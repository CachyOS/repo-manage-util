#include "packages_suggest_handler.hpp"
#include "http_utils.hpp"
#include "queries.hpp"

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
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wnull-dereference"
#pragma GCC diagnostic ignored "-Wuseless-cast"
#pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/formats/serialize/common_containers.hpp>
#include <userver/http/common_headers.hpp>
#include <userver/server/handlers/exceptions.hpp>
#include <userver/storages/postgres/cluster.hpp>
#include <userver/utils/from_string.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {

auto get_arg_helper(std::string_view arg) noexcept -> std::optional<std::int32_t> {
    if (arg.empty()) {
        return std::nullopt;
    }
    return userver::utils::FromString<std::int32_t>(arg);
}

}  // namespace

namespace service::pg {

userver::formats::json::Value PackagesSuggestHandler::HandleRequestJsonThrow(
    const userver::server::http::HttpRequest& request,
    const userver::formats::json::Value&,
    userver::server::request::RequestContext&) const {

    // set http headers
    auto& response = request.GetHttpResponse();
    response.SetHeader(userver::http::headers::kContentType, "application/x-suggestions+json");
    http::utils::set_response_http_headers(request);

    const auto& limit = get_arg_helper(request.GetArg("limit")).value_or(10);
    const auto& query = request.GetPathArg("query");

    using userver::storages::postgres::ClusterHostType;
    auto result = pg_cluster_->Execute(ClusterHostType::kSlave, query::kSelectTopPackageNames, limit, query);

    auto res_arr = userver::formats::json::ValueBuilder(userver::formats::json::Type::kArray);
    res_arr.PushBack(query);

    auto js_arr = userver::formats::json::ValueBuilder(userver::formats::json::Type::kArray);
    for (auto row : result.AsSetOf<std::string>(userver::storages::postgres::kFieldTag)) {
        js_arr.PushBack(std::move(row));
    }
    res_arr.PushBack(js_arr.ExtractValue());

    return res_arr.ExtractValue();
}

}  // namespace service::pg
