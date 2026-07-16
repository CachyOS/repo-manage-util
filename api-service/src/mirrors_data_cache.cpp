#include "mirrors_data_cache.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <ranges>
#include <string>
#include <string_view>
#include <tuple>
#include <utility>
#include <vector>

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#pragma clang diagnostic ignored "-Wold-style-cast"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wnull-dereference"
#pragma GCC diagnostic ignored "-Wuseless-cast"
#pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/clients/http/component.hpp>
#include <userver/clients/http/response.hpp>
#include <userver/engine/async.hpp>
#include <userver/engine/get_all.hpp>
#include <userver/engine/task/task_with_result.hpp>
#include <userver/http/url.hpp>
#include <userver/logging/log.hpp>
#include <userver/utils/from_string.hpp>
#include <userver/utils/text_light.hpp>
#include <userver/yaml_config/merge_schemas.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {

namespace text = userver::utils::text;
using service::mirrors::MirrorStatus;
using service::mirrors::RepoCheck;
using service::mirrors::RepoStatus;

inline constexpr std::string_view kLastUpdateEndpoint = "lastupdate";

// Builds `<base_url>/<repo_path>/lastupdate`.
auto join_timestamp_url(std::string_view base_url, std::string_view repo_path) -> std::string {
    std::string url{base_url};
    if (url.empty() || url.back() != '/') {
        url.push_back('/');
    }

    const auto segments = text::SplitIntoStringViewVector(repo_path, "/")
        | std::ranges::views::transform(userver::http::UrlEncodePathSegment)
        | std::ranges::to<std::vector<std::string>>();
    url.append(text::Join(segments, "/"));
    url.push_back('/');
    url.append(kLastUpdateEndpoint);
    return url;
}

struct MirrorAggregate {
    MirrorStatus overall_status = MirrorStatus::kError;
    std::optional<std::int64_t> average_lag_seconds;
    std::optional<std::int64_t> delay_seconds;
};

// Derives the mirror-level status and lag summary purely from its per-repo checks.
auto classify_mirror(const std::vector<RepoCheck>& checks) -> MirrorAggregate {
    std::size_t synced{};
    std::size_t errored{};
    std::int64_t measured_lags{};
    std::int64_t lag_sum{};
    std::int64_t lag_max{};
    for (const auto& check : checks) {
        if (check.status == RepoStatus::kSynced) {
            ++synced;
        } else if (check.status == RepoStatus::kError) {
            ++errored;
        }
        if (check.sync_lag_seconds) {
            // a mirror ahead of the baseline is not stale, so clamp to zero
            const auto lag = std::max<std::int64_t>(0, *check.sync_lag_seconds);
            lag_sum += lag;
            lag_max = std::max(lag_max, lag);
            ++measured_lags;
        }
    }

    MirrorAggregate aggregate;
    const auto total = checks.size();
    if (errored == total) {
        aggregate.overall_status = MirrorStatus::kError;
    } else if (synced == total) {
        aggregate.overall_status = MirrorStatus::kHealthy;
    } else if (synced == 0) {
        aggregate.overall_status = MirrorStatus::kOutOfSync;
    } else {
        aggregate.overall_status = MirrorStatus::kPartial;
    }

    if (measured_lags != 0) {
        aggregate.delay_seconds       = lag_max;
        aggregate.average_lag_seconds = std::llround(
            static_cast<double>(lag_sum) / static_cast<double>(measured_lags));
    }
    return aggregate;
}

}  // namespace

namespace service::mirrors {

MirrorsDataCache::MirrorsDataCache(
    const userver::components::ComponentConfig& config,
    const userver::components::ComponentContext& context)
  : CachingComponentBase(config, context),
    http_client_(context.FindComponent<userver::components::HttpClient>().GetHttpClient()),
    mirrorlist_url_(config["mirrorlist-url"].As<std::string>()),
    primary_mirror_url_(config["primary-mirror-url"].As<std::string>()),
    repo_paths_(config["repo-paths"].As<std::vector<std::string>>()),
    request_timeout_(config["request-timeout"].As<std::chrono::milliseconds>()),
    sync_tolerance_(config["sync-tolerance"].As<std::chrono::seconds>()) { }

void MirrorsDataCache::Update(
    userver::cache::UpdateType,
    const std::chrono::system_clock::time_point&,
    const std::chrono::system_clock::time_point&,
    userver::cache::UpdateStatisticsScope& stats_scope) {
    auto mirrors_data          = ComputeMirrorsData();
    const auto documents_count = mirrors_data.mirrors.size();
    Set(std::move(mirrors_data));
    stats_scope.Finish(documents_count);
}

std::vector<MirrorMetadata> MirrorsDataCache::FetchMirrorlist() const {
    // on failure userver keeps the last good cache data
    const auto response = http_client_.CreateRequest()
                              .get(mirrorlist_url_)
                              .http_version(userver::http::HttpVersion::k11)
                              .headers({{"Connection", "close"}})
                              .retry(2)
                              .timeout(request_timeout_)
                              .perform();
    response->raise_for_status();
    return ParseMirrorlist(response->body_view());
}

std::optional<MirrorsDataCache::Timestamp> MirrorsDataCache::FetchRepoTimestamp(
    std::string_view base_url, std::string_view repo_path) const {
    try {
        const auto response = http_client_.CreateRequest()
                                  .get(join_timestamp_url(base_url, repo_path))
                                  .http_version(userver::http::HttpVersion::k11)
                                  .headers({{"Connection", "close"}})
                                  .timeout(request_timeout_)
                                  .perform();
        if (!response->IsOk()) {
            return std::nullopt;
        }

        const auto parsed = userver::utils::FromStringNoThrow<std::int64_t>(
            text::TrimView(response->body_view()));
        if (!parsed.has_value()) {
            return std::nullopt;
        }
        return Timestamp{std::chrono::microseconds{parsed.value()}};
    } catch (const std::exception& ex) {
        LOG_DEBUG("Failed to fetch repo timestamp for '{}' from '{}': {}",
            repo_path, base_url, ex.what());
        return std::nullopt;
    }
}

MirrorsData MirrorsDataCache::ComputeMirrorsData() const {
    const auto mirror_metadata_list = FetchMirrorlist();

    auto baseline_tasks = repo_paths_ | std::ranges::views::transform([this](const auto& repo_path) {
        return userver::engine::AsyncNoTracing(
            [this, &repo_path] { return FetchRepoTimestamp(primary_mirror_url_, repo_path); });
    }) | std::ranges::to<std::vector>();

    const auto baseline_timestamps = userver::engine::GetAll(baseline_tasks);
    BaselineMap baseline_map;
    baseline_map.reserve(repo_paths_.size());
    for (const auto& [repo_path, timestamp] : std::ranges::views::zip(repo_paths_, baseline_timestamps)) {
        baseline_map.try_emplace(repo_path, timestamp);
    }

    auto mirror_tasks = mirror_metadata_list | std::ranges::views::transform([this, &baseline_map](const auto& mirror_metadata) {
        return userver::engine::AsyncNoTracing(
            [this, &mirror_metadata, &baseline_map] {
                return BuildMirrorEntry(mirror_metadata, baseline_map);
            });
    }) | std::ranges::to<std::vector>();

    MirrorsData result{.mirrors = userver::engine::GetAll(mirror_tasks)};

    const auto sort_key = [](const MirrorEntry& m) {
        return std::tuple(
            m.out_of_date,
            m.tier,
            !m.last_sync.has_value(),
            -m.last_sync.value_or(Timestamp{}).time_since_epoch().count(),
            std::string_view{m.url});
    };
    std::ranges::sort(result.mirrors, std::less<>{}, sort_key);

    return result;
}

MirrorEntry MirrorsDataCache::BuildMirrorEntry(
    const MirrorMetadata& mirror_metadata, const BaselineMap& baseline_map) const {
    auto timestamp_tasks = repo_paths_ | std::ranges::views::transform([this, &mirror_metadata](const auto& repo_path) {
        return userver::engine::AsyncNoTracing(
            [this, &mirror_metadata, &repo_path] {
                return FetchRepoTimestamp(mirror_metadata.url, repo_path);
            });
    }) | std::ranges::to<std::vector>();

    MirrorEntry mirror{
        .out_of_date         = false,
        .tier                = mirror_metadata.tier,
        .country_code        = mirror_metadata.country_code,
        .url                 = mirror_metadata.url,
        .last_sync           = std::nullopt,
        .checks              = {},
        .average_lag_seconds = std::nullopt,
        .delay_seconds       = std::nullopt,
    };

    mirror.checks.reserve(repo_paths_.size());

    const auto timestamps = userver::engine::GetAll(timestamp_tasks);
    for (const auto& [repo_path, timestamp] : std::ranges::views::zip(repo_paths_, timestamps)) {
        RepoCheck check{
            .path             = repo_path,
            .last_updated     = timestamp,
            .sync_lag_seconds = std::nullopt,
            .status           = RepoStatus::kError,
        };

        if (timestamp.has_value()) {
            if (!mirror.last_sync.has_value() || *timestamp > *mirror.last_sync) {
                mirror.last_sync = timestamp;
            }

            const auto baseline_it = baseline_map.find(repo_path);
            if (baseline_it != baseline_map.end() && baseline_it->second.has_value()) {
                const auto raw_lag     = *baseline_it->second - *timestamp;
                check.sync_lag_seconds = std::chrono::duration_cast<std::chrono::seconds>(raw_lag).count();
                check.status           = raw_lag > sync_tolerance_ ? RepoStatus::kOutOfSync : RepoStatus::kSynced;
            } else {
                // nothing to compare against
                check.status = RepoStatus::kSynced;
            }
        }

        if (check.status != RepoStatus::kSynced) {
            mirror.out_of_date = true;
        }
        mirror.checks.push_back(std::move(check));
    }

    if (!mirror.last_sync.has_value()) {
        mirror.out_of_date = true;
    }

    const auto aggregate       = classify_mirror(mirror.checks);
    mirror.overall_status      = aggregate.overall_status;
    mirror.average_lag_seconds = aggregate.average_lag_seconds;
    mirror.delay_seconds       = aggregate.delay_seconds;
    return mirror;
}

userver::yaml_config::Schema MirrorsDataCache::GetStaticConfigSchema() {
    return userver::yaml_config::MergeSchemas<CachingComponentBase>(R"(
type: object
description: Cached mirror health data
additionalProperties: false
properties:
    mirrorlist-url:
        type: string
        description: URL that returns the mirror list in plain text
    primary-mirror-url:
        type: string
        description: Base URL for baseline lastupdate checks
    repo-paths:
        type: array
        description: List of repo paths to evaluate under each mirror base URL
        items:
            type: string
            description: Repo path appended to a mirror base URL when fetching lastupdate
    request-timeout:
        type: string
        description: Timeout (duration, e.g. 2s) for outbound mirror requests
    sync-tolerance:
        type: string
        description: Max lag (duration, e.g. 1h) still considered synced
)");
}

}  // namespace service::mirrors
