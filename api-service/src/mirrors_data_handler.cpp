#include "mirrors_data_handler.hpp"

#include "http_utils.hpp"
#include "mirrors_data_cache.hpp"

#include <optional>

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

#include <userver/formats/common/type.hpp>
#include <userver/formats/json/value_builder.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace
{
    template <typename T>
    void set_optional_field(
        userver::formats::json::ValueBuilder& builder,
        const std::string_view key,
        const std::optional<T>& value)
    {
        const auto key_name = std::string{key};
        if (value)
        {
            builder[key_name] = *value;
            return;
        }

        builder[key_name] = userver::formats::json::ValueBuilder{userver::formats::common::Type::kNull};
    }
} // namespace

namespace service::mirrors
{
    MirrorsDataHandler::MirrorsDataHandler(
        const userver::components::ComponentConfig& config,
        const userver::components::ComponentContext& component_context)
        : HttpHandlerJsonBase(config, component_context),
          cache_(component_context.FindComponent<MirrorsDataCache>())
    {
    }

    userver::formats::json::Value MirrorsDataHandler::HandleRequestJsonThrow(
        const userver::server::http::HttpRequest& request,
        const userver::formats::json::Value&,
        userver::server::request::RequestContext&) const
    {
        using enum userver::formats::common::Type;
        http::utils::set_response_http_headers(request);

        const auto mirrors_data = cache_.Get();

        auto response = userver::formats::json::ValueBuilder(kObject);
        response["baselines"] = userver::formats::json::ValueBuilder(kArray);
        response["mirrors"] = userver::formats::json::ValueBuilder(kArray);

        for (const auto& baseline : mirrors_data->baselines)
        {
            auto baseline_json = userver::formats::json::ValueBuilder(kObject);
            baseline_json["path"] = baseline.path;
            set_optional_field(baseline_json, "timestamp", baseline.timestamp);
            response["baselines"].PushBack(std::move(baseline_json));
        }

        for (const auto& mirror : mirrors_data->mirrors)
        {
            auto mirror_json = userver::formats::json::ValueBuilder(kObject);
            mirror_json["name"] = mirror.name;
            mirror_json["url"] = mirror.url;
            mirror_json["checks"] = userver::formats::json::ValueBuilder(kArray);
            set_optional_field(mirror_json, "averageLagSeconds", mirror.average_lag_seconds);
            mirror_json["overallStatus"] = mirror.overall_status;

            for (const auto& check : mirror.checks)
            {
                auto check_json = userver::formats::json::ValueBuilder(kObject);
                check_json["path"] = check.path;
                set_optional_field(check_json, "lastUpdated", check.last_updated);
                check_json["status"] = check.status;
                set_optional_field(check_json, "syncLagSeconds", check.sync_lag_seconds);
                mirror_json["checks"].PushBack(std::move(check_json));
            }

            response["mirrors"].PushBack(std::move(mirror_json));
        }

        return response.ExtractValue();
    }
} // namespace service::mirrors
