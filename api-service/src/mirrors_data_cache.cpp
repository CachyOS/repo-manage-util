#include "mirrors_data_cache.hpp"

#include <algorithm>
#include <charconv>
#include <cctype>
#include <cstdint>
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
    enum class PendingServerState : std::uint8_t
    {
        kNone,
        kEnabled,
        kDisabled,
    };

    struct PendingMirrorMetadata
    {
        std::string country_code;
        int tier{2};
    };

    struct ParsedServerDirective
    {
        bool matched{false};
        std::optional<std::string_view> url;
    };

    struct RepoTimestampResult
    {
        std::string path;
        std::optional<std::int64_t> timestamp;
    };

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

    auto normalize_mirror_base_url(const std::string_view text) -> std::optional<std::string>
    {
        auto normalized = std::string{trim_view(text)};
        if (normalized.empty())
        {
            return std::nullopt;
        }

        if (normalized.front() == '=')
        {
            normalized.erase(0, 1);
            normalized = std::string{trim_view(normalized)};
        }

        if (!normalized.starts_with("https://") && !normalized.starts_with("http://"))
        {
            return std::nullopt;
        }

        for (const auto suffix : {
                 std::string_view{"/repo/$arch/$repo"},
                 std::string_view{"/$repo/$arch"},
                 std::string_view{"/$arch/$repo"},
                 std::string_view{"/$repo"},
                 std::string_view{"/$arch"},
             })
        {
            if (normalized.ends_with(suffix))
            {
                normalized.erase(normalized.size() - suffix.size());
                break;
            }
        }

        while (!normalized.empty() && normalized.back() == '/')
        {
            normalized.pop_back();
        }

        normalized.push_back('/');

        return normalized;
    }

    auto parse_value_after_key(const std::string_view text, const std::string_view key)
        -> std::optional<std::string_view>
    {
        const auto key_pos = text.find(key);
        if (key_pos == std::string_view::npos)
        {
            return std::nullopt;
        }

        auto value = trim_view(text.substr(key_pos + key.size()));
        if (value.empty())
        {
            return std::nullopt;
        }

        std::size_t token_size = 0;
        while (token_size < value.size() && std::isspace(static_cast<unsigned char>(value[token_size])) == 0)
        {
            ++token_size;
        }

        value = value.substr(0, token_size);
        while (!value.empty() && std::ispunct(static_cast<unsigned char>(value.back())) != 0 && value.back() != '-')
        {
            value.remove_suffix(1);
        }

        if (value.empty())
        {
            return std::nullopt;
        }

        return value;
    }

    void update_metadata_from_comment(const std::string_view comment, PendingMirrorMetadata& metadata)
    {
        if (const auto tier_value = parse_value_after_key(comment, "tier="); tier_value.has_value())
        {
            if (const auto parsed_tier = parse_integer(*tier_value);
                parsed_tier.has_value() && (*parsed_tier == 1 || *parsed_tier == 2))
            {
                metadata.tier = static_cast<int>(*parsed_tier);
            }
        }

        if (const auto country_code_value = parse_value_after_key(comment, "code="); country_code_value.has_value())
        {
            metadata.country_code.clear();
            metadata.country_code.reserve(country_code_value->size());
            for (const auto symbol : *country_code_value)
            {
                metadata.country_code.push_back(
                    static_cast<char>(std::toupper(static_cast<unsigned char>(symbol))));
            }
        }
    }

    auto try_extract_url(const std::string_view text) -> std::optional<std::string_view>
    {
        auto candidate = trim_view(text);
        if (candidate.empty())
        {
            return std::nullopt;
        }

        if (candidate.front() == '=')
        {
            candidate.remove_prefix(1);
            candidate = trim_view(candidate);
        }

        if (!candidate.starts_with("https://") && !candidate.starts_with("http://"))
        {
            return std::nullopt;
        }

        return candidate;
    }

    auto parse_server_directive(const std::string_view text) -> ParsedServerDirective
    {
        auto candidate = trim_view(text);
        if (!candidate.starts_with("Server"))
        {
            return {};
        }

        candidate.remove_prefix(std::string_view{"Server"}.size());
        candidate = trim_view(candidate);
        if (candidate.empty())
        {
            return {.matched = true, .url = std::nullopt};
        }

        if (candidate.front() == '=')
        {
            candidate.remove_prefix(1);
            candidate = trim_view(candidate);
        }

        if (candidate.empty())
        {
            return {.matched = true, .url = std::nullopt};
        }

        return {.matched = true, .url = candidate};
    }

    auto join_timestamp_url(const std::string_view base_url, const std::string_view repo_path) -> std::string
    {
        std::string url{base_url};
        if (!url.ends_with('/'))
        {
            url.push_back('/');
        }
        url.append(repo_path);
        url.push_back('/');
        url.append("lastupdate");
        return url;
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
        const auto documents_count = mirrors_data.mirrors.size();
        Set(std::move(mirrors_data));
        stats_scope.Finish(documents_count);
    }

    auto MirrorsDataCache::FetchMirrorlist() const -> std::vector<MirrorMetadata>
    {
        std::vector<MirrorMetadata> mirrors;

        try
        {
            const auto response = http_client_.CreateRequest().get(mirrorlist_url_).timeout(request_timeout_).perform();
            if (!response->IsOk())
            {
                LOG_DEBUG() << "Mirror list request failed with status " << response->status_code();
                return mirrors;
            }

            std::unordered_set<std::string> seen_urls;
            PendingMirrorMetadata current_metadata;
            auto pending_server_state = PendingServerState::kNone;

            const auto reset_pending_entry = [&]()
            {
                current_metadata = PendingMirrorMetadata{};
                pending_server_state = PendingServerState::kNone;
            };

            const auto append_mirror = [&](const std::string_view raw_url)
            {
                const auto normalized = normalize_mirror_base_url(raw_url);
                if (!normalized)
                {
                    return;
                }

                if (seen_urls.insert(*normalized).second)
                {
                    mirrors.push_back(MirrorMetadata{
                        .country_code = current_metadata.country_code,
                        .url = *normalized,
                        .tier = current_metadata.tier,
                    });
                }
                reset_pending_entry();
            };

            for (const auto raw_line : split_lines(response->body_view()))
            {
                auto line = trim_view(raw_line);
                if (line.empty())
                {
                    continue;
                }

                const auto commented_out = line.front() == '#';
                auto content = line;
                if (commented_out)
                {
                    while (!content.empty() && content.front() == '#')
                    {
                        content.remove_prefix(1);
                    }
                    content = trim_view(content);
                }

                if (pending_server_state != PendingServerState::kNone)
                {
                    if (const auto pending_url = try_extract_url(content); pending_url.has_value())
                    {
                        if (pending_server_state == PendingServerState::kEnabled && !commented_out)
                        {
                            append_mirror(*pending_url);
                        }
                        else
                        {
                            reset_pending_entry();
                        }
                        continue;
                    }

                    reset_pending_entry();
                }

                if (const auto server = parse_server_directive(content); server.matched)
                {
                    if (server.url.has_value())
                    {
                        if (!commented_out)
                        {
                            append_mirror(*server.url);
                        }
                        else
                        {
                            reset_pending_entry();
                        }
                    }
                    else
                    {
                        pending_server_state =
                            commented_out ? PendingServerState::kDisabled : PendingServerState::kEnabled;
                    }
                    continue;
                }

                if (commented_out)
                {
                    update_metadata_from_comment(content, current_metadata);
                    continue;
                }

                if (const auto standalone_url = try_extract_url(content); standalone_url.has_value())
                {
                    append_mirror(*standalone_url);
                }
            }
        }
        catch (const std::exception& ex)
        {
            LOG_DEBUG() << "Failed to fetch mirror list: " << ex;
        }

        return mirrors;
    }

    auto MirrorsDataCache::FetchRepoTimestamp(const std::string_view base_url, const std::string_view repo_path)
    const -> std::optional<std::int64_t>
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

        const auto mirror_metadata_list = FetchMirrorlist();

        std::vector<userver::engine::TaskWithResult<RepoTimestampResult>> baseline_tasks;
        baseline_tasks.reserve(repo_paths_.size());
        for (const auto& repo_path : repo_paths_)
        {
            baseline_tasks.push_back(userver::engine::AsyncNoSpan([this, repo_path]
            {
                return RepoTimestampResult{
                    .path = repo_path,
                    .timestamp = FetchRepoTimestamp(primary_mirror_url_, repo_path),
                };
            }));
        }

        BaselineMap baseline_map;
        baseline_map.reserve(repo_paths_.size());
        for (auto& task : baseline_tasks)
        {
            auto baseline = task.Get();
            baseline_map.try_emplace(baseline.path, baseline.timestamp);
        }

        std::vector<userver::engine::TaskWithResult<MirrorEntry>> mirror_tasks;
        mirror_tasks.reserve(mirror_metadata_list.size());
        for (const auto& mirror_metadata : mirror_metadata_list)
        {
            mirror_tasks.push_back(userver::engine::AsyncNoSpan([this, mirror_metadata, &baseline_map]
            {
                return BuildMirrorEntry(mirror_metadata, baseline_map);
            }));
        }

        result.mirrors.reserve(mirror_tasks.size());
        for (auto& task : mirror_tasks)
        {
            result.mirrors.push_back(task.Get());
        }

        std::ranges::sort(result.mirrors, [](const MirrorEntry& left, const MirrorEntry& right)
        {
            if (left.out_of_date != right.out_of_date)
            {
                return !left.out_of_date && right.out_of_date;
            }

            if (left.tier != right.tier)
            {
                return left.tier < right.tier;
            }

            if (!left.last_sync.has_value() && !right.last_sync.has_value())
            {
                return left.url < right.url;
            }
            if (!left.last_sync.has_value())
            {
                return false;
            }
            if (!right.last_sync.has_value())
            {
                return true;
            }
            if (*left.last_sync != *right.last_sync)
            {
                return *left.last_sync > *right.last_sync;
            }

            return left.url < right.url;
        });

        return result;
    }

    auto MirrorsDataCache::BuildMirrorEntry(const MirrorMetadata& mirror_metadata, const BaselineMap& baseline_map)
    const -> MirrorEntry
    {
        std::vector<userver::engine::TaskWithResult<RepoTimestampResult>> timestamp_tasks;
        timestamp_tasks.reserve(repo_paths_.size());

        for (const auto& repo_path : repo_paths_)
        {
            timestamp_tasks.push_back(userver::engine::AsyncNoSpan([this, mirror_url = mirror_metadata.url, repo_path]
            {
                return RepoTimestampResult{
                    .path = repo_path,
                    .timestamp = FetchRepoTimestamp(mirror_url, repo_path),
                };
            }));
        }

        MirrorEntry mirror{
            .country_code = mirror_metadata.country_code,
            .url = mirror_metadata.url,
            .out_of_date = false,
            .last_sync = std::nullopt,
            .tier = mirror_metadata.tier,
        };

        bool has_missing_timestamp = false;
        bool has_stale_timestamp = false;

        for (auto& task : timestamp_tasks)
        {
            const auto repo_result = task.Get();
            if (!repo_result.timestamp.has_value())
            {
                has_missing_timestamp = true;
                continue;
            }

            if (!mirror.last_sync.has_value() || *repo_result.timestamp < *mirror.last_sync)
            {
                mirror.last_sync = repo_result.timestamp;
            }

            if (const auto baseline_it = baseline_map.find(repo_result.path); baseline_it != baseline_map.end())
            {
                if (const auto& baseline_timestamp = baseline_it->second;
                    baseline_timestamp.has_value() &&
                    (*baseline_timestamp - *repo_result.timestamp) > sync_tolerance_.count())
                {
                    has_stale_timestamp = true;
                }
            }
        }

        mirror.out_of_date = has_missing_timestamp || has_stale_timestamp || !mirror.last_sync.has_value();

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
