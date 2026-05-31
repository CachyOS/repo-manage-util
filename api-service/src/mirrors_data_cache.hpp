#pragma once

#include <chrono>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#elifdef __GNUC__
#pragma GCC diagnostic push
#endif

#include <userver/cache/caching_component_base.hpp>
#include <userver/clients/http/client.hpp>
#include <userver/components/component_config.hpp>

#include <userver/yaml_config/schema.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace service::mirrors
{
    struct MirrorEntry
    {
        std::string country_code;
        std::string url;
        bool out_of_date;
        std::optional<std::int64_t> last_sync;
        int tier;
    };

    struct MirrorsData
    {
        std::vector<MirrorEntry> mirrors;
    };

    class MirrorsDataCache final : public userver::components::CachingComponentBase<MirrorsData>
    {
    public:
        static constexpr std::string_view kName = "mirrors-data-cache";

        MirrorsDataCache(
            const userver::components::ComponentConfig& config,
            const userver::components::ComponentContext& context);

        void Update(
            userver::cache::UpdateType type,
            const std::chrono::system_clock::time_point& last_update,
            const std::chrono::system_clock::time_point& now,
            userver::cache::UpdateStatisticsScope& stats_scope) override;

        static userver::yaml_config::Schema GetStaticConfigSchema();

    private:
        struct MirrorMetadata
        {
            std::string country_code;
            std::string url;
            int tier;
        };

        using BaselineMap = std::unordered_map<std::string, std::optional<std::int64_t>>;

        [[nodiscard]] auto FetchMirrorlist() const -> std::vector<MirrorMetadata>;
        [[nodiscard]] auto FetchRepoTimestamp(std::string_view base_url, std::string_view repo_path) const
            -> std::optional<std::int64_t>;
        [[nodiscard]] auto ComputeMirrorsData() const -> MirrorsData;
        [[nodiscard]] auto BuildMirrorEntry(
            const MirrorMetadata& mirror_metadata,
            const BaselineMap& baseline_map) const -> MirrorEntry;

        userver::clients::http::Client& http_client_;
        std::string mirrorlist_url_;
        std::string primary_mirror_url_;
        std::vector<std::string> repo_paths_;
        std::chrono::milliseconds request_timeout_;
        std::chrono::seconds sync_tolerance_;
    };
} // namespace service::mirrors
