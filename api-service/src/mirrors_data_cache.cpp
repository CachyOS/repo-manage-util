#include "mirrors_data_cache.hpp"

#include <algorithm>
#include <charconv>
#include <cctype>
#include <cmath>
#include <string>
#include <string_view>
#include <unordered_set>
#include <utility>

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#pragma clang diagnostic ignored "-Wold-style-cast"
#elifdef __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wnull-dereference"
#pragma GCC diagnostic ignored "-Wuseless-cast"
#pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/clients/http/component.hpp>
#include <userver/clients/http/response.hpp>
#include <userver/engine/async.hpp>
#include <userver/engine/task/task_with_result.hpp>
#include <userver/logging/log.hpp>
#include <userver/yaml_config/merge_schemas.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace
{
    constexpr std::string_view kStatusSynced = "synced";
    constexpr std::string_view kStatusOutOfSync = "out-of-sync";
    constexpr std::string_view kStatusError = "error";

    constexpr std::string_view kOverallHealthy = "healthy";
    constexpr std::string_view kOverallPartial = "partial";
    constexpr std::string_view kOverallOutOfSync = "out-of-sync";
    constexpr std::string_view kOverallError = "error";

    constexpr auto trim_view(std::string_view text) noexcept -> std::string_view
    {
        while (!text.empty() && std::isspace(static_cast<unsigned char>(text.front())) != 0)
        {
            text.remove_prefix(1);
        }
        while (!text.empty() && std::isspace(static_cast<unsigned char>(text.back())) != 0)
        {
            text.remove_suffix(1);
        }
        return text;
    }

    auto split_lines(std::string_view text) -> std::vector<std::string_view>
    {
        std::vector<std::string_view> lines;
        while (!text.empty())
        {
            const auto newline_pos = text.find('\n');
            if (newline_pos == std::string_view::npos)
            {
                lines.push_back(text);
                break;
            }

            lines.push_back(text.substr(0, newline_pos));
            text.remove_prefix(newline_pos + 1);
        }
        return lines;
    }

    auto parse_integer(std::string_view text) noexcept -> std::optional<std::int64_t>
    {
        text = trim_view(text);
        if (text.empty())
        {
            return std::nullopt;
        }

        std::int64_t value{};
        const auto* begin = text.data();
        const auto* end = begin + text.size();
        if (const auto result = std::from_chars(begin, end, value);
            result.ec != std::errc{} || result.ptr != end)
        {
            return std::nullopt;
        }

        return value;
    }

    auto normalize_mirror_base_url(std::string_view line) -> std::optional<std::string>
    {
        auto normalized = std::string{trim_view(line)};
        if (normalized.empty() || normalized.front() == '#')
        {
            return std::nullopt;
        }

        if (const auto equals_pos = normalized.find('='); equals_pos != std::string::npos)
        {
            if (const auto key = trim_view(std::string_view{normalized}.substr(0, equals_pos));
                key != "Server")
            {
                return std::nullopt;
            }
            normalized = std::string{trim_view(std::string_view{normalized}.substr(equals_pos + 1))};
        }

        if (!normalized.starts_with("http://") && !normalized.starts_with("https://"))
        {
            return std::nullopt;
        }

        for (const auto suffix : {
                 std::string_view{"/$repo/$arch"},
                 std::string_view{"/$arch/$repo"},
                 std::string_view{"/$repo"},
                 std::string_view{"/$arch"},
             })
        {
            if (const auto pos = normalized.find(suffix); pos != std::string::npos)
            {
                normalized.erase(pos);
            }
        }

        while (normalized.size() > 1 && normalized.back() == '/')
        {
            normalized.pop_back();
        }

        return normalized;
    }

    auto join_timestamp_url(std::string_view base_url, std::string_view repo_path) -> std::string
    {
        std::string url{base_url};
        if (!url.ends_with('/'))
        {
            url.push_back('/');
        }
        url.append(repo_path);
        if (!url.ends_with('/'))
        {
            url.push_back('/');
        }
        url.append("lastupdate");
        return url;
    }

    auto extract_hostname(std::string_view url) -> std::string
    {
        if (const auto scheme_pos = url.find("://"); scheme_pos != std::string_view::npos)
        {
            url.remove_prefix(scheme_pos + 3);
        }

        const auto slash_pos = url.find('/');
        auto authority = url.substr(0, slash_pos);
        if (const auto userinfo_pos = authority.rfind('@'); userinfo_pos != std::string_view::npos)
        {
            authority.remove_prefix(userinfo_pos + 1);
        }

        if (!authority.empty() && authority.front() == '[')
        {
            const auto end_pos = authority.find(']');
            if (end_pos != std::string_view::npos)
            {
                return std::string{authority.substr(0, end_pos + 1)};
            }
        }

        if (const auto colon_pos = authority.find(':');
            colon_pos != std::string_view::npos)
        {
            authority = authority.substr(0, colon_pos);
        }

        return std::string{authority};
    }

    constexpr auto overall_status_rank(std::string_view status) noexcept -> int
    {
        if (status == kOverallHealthy)
        {
            return 0;
        }
        if (status == kOverallPartial)
        {
            return 1;
        }
        if (status == kOverallOutOfSync)
        {
            return 2;
        }
        return 3;
    }
} // namespace

namespace service::mirrors
{
    MirrorsDataCache::MirrorsDataCache(
        const userver::components::ComponentConfig& config,
        const userver::components::ComponentContext& context)
        : CachingComponentBase(config, context),
          http_client_(context.FindComponent<userver::components::HttpClient>().GetHttpClient()),
          mirrorlist_url_(config["mirrorlist-url"].As<std::string>()),
          primary_mirror_url_(config["primary-mirror-url"].As<std::string>()),
          repo_paths_(config["repo-paths"].As<std::vector<std::string>>()),
          request_timeout_(std::chrono::milliseconds{config["request-timeout-ms"].As<int>()}),
          sync_tolerance_(std::chrono::seconds{config["sync-tolerance-seconds"].As<int>()})
    {
    }

    void MirrorsDataCache::Update(
        userver::cache::UpdateType,
        const std::chrono::system_clock::time_point&,
        const std::chrono::system_clock::time_point&,
        userver::cache::UpdateStatisticsScope& stats_scope)
    {
        auto mirrors_data = ComputeMirrorsData();
        const auto documents_count = mirrors_data.baselines.size() + mirrors_data.mirrors.size();
        Set(std::move(mirrors_data));
        stats_scope.Finish(documents_count);
    }

    auto MirrorsDataCache::FetchMirrorlist() const -> std::vector<std::string>
    {
        std::vector<std::string> mirror_urls;

        try
        {
            auto response = http_client_.CreateRequest().get(mirrorlist_url_).timeout(request_timeout_).perform();
            if (!response->IsOk())
            {
                LOG_DEBUG() << "Mirror list request failed with status " << response->status_code();
                return mirror_urls;
            }

            std::unordered_set<std::string> seen_urls;
            for (const auto line : split_lines(response->body_view()))
            {
                const auto normalized = normalize_mirror_base_url(line);
                if (!normalized)
                {
                    continue;
                }
                if (seen_urls.insert(*normalized).second)
                {
                    mirror_urls.push_back(*normalized);
                }
            }
        }
        catch (const std::exception& ex)
        {
            LOG_DEBUG() << "Failed to fetch mirror list: " << ex;
        }

        return mirror_urls;
    }

    auto MirrorsDataCache::FetchRepoTimestamp(std::string_view base_url, std::string_view repo_path) const
        -> std::optional<std::int64_t>
    {
        try
        {
            const auto response =
                http_client_.CreateRequest()
                            .get(join_timestamp_url(base_url, repo_path))
                            .timeout(request_timeout_)
                            .perform();
            if (!response->IsOk())
            {
                return std::nullopt;
            }

            const auto parsed_timestamp = parse_integer(response->body_view());
            if (!parsed_timestamp.has_value())
            {
                return std::nullopt;
            }

            return *parsed_timestamp / 1000;
        }
        catch (const std::exception& ex)
        {
            LOG_DEBUG() << "Failed to fetch repo timestamp for '" << repo_path << "' from '" << base_url
                << "': " << ex;
            return std::nullopt;
        }
    }

    auto MirrorsDataCache::ComputeMirrorsData() const -> MirrorsData
    {
        MirrorsData result;

        const auto mirror_urls = FetchMirrorlist();

        std::vector<userver::engine::TaskWithResult<BaselineEntry>> baseline_tasks;
        baseline_tasks.reserve(repo_paths_.size());
        for (const auto& repo_path : repo_paths_)
        {
            baseline_tasks.push_back(userver::engine::AsyncNoSpan([this, repo_path]
            {
                return BaselineEntry{
                    .path = repo_path,
                    .timestamp = FetchRepoTimestamp(primary_mirror_url_, repo_path),
                };
            }));
        }

        result.baselines.reserve(baseline_tasks.size());
        BaselineMap baseline_map;
        baseline_map.reserve(repo_paths_.size());
        for (auto& task : baseline_tasks)
        {
            auto baseline = task.Get();
            baseline_map.try_emplace(baseline.path, baseline.timestamp);
            result.baselines.push_back(std::move(baseline));
        }

        std::vector<userver::engine::TaskWithResult<MirrorResult>> mirror_tasks;
        mirror_tasks.reserve(mirror_urls.size());
        for (const auto& mirror_url : mirror_urls)
        {
            mirror_tasks.push_back(userver::engine::AsyncNoSpan([this, mirror_url, &baseline_map]
            {
                return BuildMirrorResult(mirror_url, baseline_map);
            }));
        }

        result.mirrors.reserve(mirror_tasks.size());
        for (auto& task : mirror_tasks)
        {
            result.mirrors.push_back(task.Get());
        }

        std::ranges::sort(result.mirrors, [](const MirrorResult& left, const MirrorResult& right)
        {
            const auto left_rank = overall_status_rank(left.overall_status);
            const auto right_rank = overall_status_rank(right.overall_status);
            if (left_rank != right_rank)
            {
                return left_rank < right_rank;
            }

            if (!left.average_lag_seconds.has_value() && !right.average_lag_seconds.has_value())
            {
                return left.url < right.url;
            }
            if (!left.average_lag_seconds.has_value())
            {
                return false;
            }
            if (!right.average_lag_seconds.has_value())
            {
                return true;
            }
            if (std::abs(*left.average_lag_seconds - *right.average_lag_seconds) > 0.0001)
            {
                return *left.average_lag_seconds < *right.average_lag_seconds;
            }
            return left.url < right.url;
        });

        return result;
    }

    auto MirrorsDataCache::BuildMirrorResult(const std::string& mirror_url, const BaselineMap& baseline_map) const
        -> MirrorResult
    {
        std::vector<userver::engine::TaskWithResult<RepoCheck>> check_tasks;
        check_tasks.reserve(repo_paths_.size());

        for (const auto& repo_path : repo_paths_)
        {
            check_tasks.push_back(userver::engine::AsyncNoSpan([this, &baseline_map, mirror_url, repo_path]
            {
                const auto mirror_timestamp = FetchRepoTimestamp(mirror_url, repo_path);
                const auto baseline_it = baseline_map.find(repo_path);
                const auto baseline_timestamp =
                    baseline_it == baseline_map.end() ? std::optional<std::int64_t>{} : baseline_it->second;

                RepoCheck check{
                    .path = repo_path,
                    .last_updated = mirror_timestamp,
                    .status = std::string{kStatusError},
                    .sync_lag_seconds = std::nullopt,
                };

                if (!mirror_timestamp.has_value())
                {
                    return check;
                }

                if (baseline_timestamp.has_value())
                {
                    const auto lag = *baseline_timestamp - *mirror_timestamp;
                    check.sync_lag_seconds = lag;
                    check.status =
                        lag <= sync_tolerance_.count() ? std::string{kStatusSynced} : std::string{kStatusOutOfSync};
                    return check;
                }

                check.status = std::string{kStatusSynced};
                return check;
            }));
        }

        MirrorResult mirror{
            .name = extract_hostname(mirror_url),
            .url = mirror_url,
            .checks = {},
            .average_lag_seconds = std::nullopt,
            .overall_status = std::string{kOverallError},
        };
        mirror.checks.reserve(check_tasks.size());

        std::size_t synced_checks = 0;
        std::size_t error_checks = 0;
        double positive_lag_sum = 0.0;
        std::size_t positive_lag_count = 0;

        for (auto& task : check_tasks)
        {
            auto check = task.Get();
            if (check.status == kStatusSynced)
            {
                ++synced_checks;
            }
            if (check.status == kStatusError)
            {
                ++error_checks;
            }
            if (check.sync_lag_seconds.has_value() && *check.sync_lag_seconds > 0)
            {
                positive_lag_sum += static_cast<double>(*check.sync_lag_seconds);
                ++positive_lag_count;
            }
            mirror.checks.push_back(std::move(check));
        }

        const auto total_checks = mirror.checks.size();
        if (const auto valid_checks = total_checks - error_checks; valid_checks == 0)
        {
            mirror.overall_status = std::string{kOverallError};
        }
        else if (synced_checks == total_checks)
        {
            mirror.overall_status = std::string{kOverallHealthy};
        }
        else
        {
            mirror.overall_status = synced_checks == 0 ? std::string{kOverallOutOfSync} : std::string{kOverallPartial};
        }

        if (positive_lag_count > 0)
        {
            mirror.average_lag_seconds = positive_lag_sum / static_cast<double>(positive_lag_count);
        }

        return mirror;
    }

    userver::yaml_config::Schema MirrorsDataCache::GetStaticConfigSchema()
    {
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
    request-timeout-ms:
        type: integer
        description: Timeout in milliseconds for outbound mirror requests
        minimum: 1
    sync-tolerance-seconds:
        type: integer
        description: Max lag in seconds still considered synced
        minimum: 0
)");
    }
} // namespace service::mirrors
