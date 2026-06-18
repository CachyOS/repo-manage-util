#include "mirrors_data_handler.hpp"

#include "http_utils.hpp"
#include "mirrors_data_cache.hpp"

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

#include <userver/formats/json/value_builder.hpp>
#include <userver/formats/serialize/common_containers.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::mirrors {

MirrorsDataHandler::MirrorsDataHandler(
    const userver::components::ComponentConfig& config,
    const userver::components::ComponentContext& component_context)
  : HttpHandlerJsonBase(config, component_context),
    cache_(component_context.FindComponent<MirrorsDataCache>()) { }

userver::formats::json::Value MirrorsDataHandler::HandleRequestJsonThrow(
    const userver::server::http::HttpRequest& request,
    const userver::formats::json::Value&,
    userver::server::request::RequestContext&) const {
    http::utils::set_response_http_headers(request);

    const auto snapshot = cache_.Get();
    auto response       = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);
    response["mirrors"] = snapshot->mirrors;
    return response.ExtractValue();
}

}  // namespace service::mirrors
