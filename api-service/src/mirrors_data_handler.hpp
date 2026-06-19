#pragma once

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#pragma clang diagnostic ignored "-Wimplicit-int-conversion"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wnon-virtual-dtor"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#endif

#include <userver/components/component_context.hpp>
#include <userver/server/handlers/http_handler_json_base.hpp>

#include <userver/formats/json/value.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::mirrors {

class MirrorsDataCache;

class MirrorsDataHandler final : public userver::server::handlers::HttpHandlerJsonBase {
 public:
    // `kName` is used as the component name in static config
    static constexpr std::string_view kName = "handler-mirrors-data";

    MirrorsDataHandler(
        const userver::components::ComponentConfig& config,
        const userver::components::ComponentContext& component_context);

    userver::formats::json::Value HandleRequestJsonThrow(
        const userver::server::http::HttpRequest& request,
        const userver::formats::json::Value& request_json,
        userver::server::request::RequestContext& ctx) const override;

 private:
    MirrorsDataCache& cache_;
};

}  // namespace service::mirrors
