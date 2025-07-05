#include "package_handler.hpp"
#include "http_utils.hpp"
#include "package_utils.hpp"
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

#include <userver/server/handlers/exceptions.hpp>
#include <userver/storages/postgres/cluster.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {
using PackageRow = service::pg::utils::PublicPackageRow;
}  // namespace

namespace service::pg {

userver::formats::json::Value PackageHandler::HandleRequestJsonThrow(
    const userver::server::http::HttpRequest& request,
    const userver::formats::json::Value&,
    userver::server::request::RequestContext&) const {
    static constexpr auto kNotFound = "Package not found!";

    // set http headers
    http::utils::set_response_http_headers(request);

    const auto& repo    = request.GetPathArg("repo");
    const auto& arch    = request.GetPathArg("arch");
    const auto& pkgname = request.GetPathArg("pkgname");

    using userver::storages::postgres::ClusterHostType;
    auto result = pg_cluster_->Execute(ClusterHostType::kSlave, query::kSelectPackage, repo, arch, pkgname);
    if (result.IsEmpty()) {
        throw userver::server::handlers::ResourceNotFound(
            userver::server::handlers::ExternalBody{kNotFound});
    }

    const auto& package_res = result.AsSingleRow<PackageRow>(userver::storages::postgres::kRowTag);
    auto js_obj             = userver::formats::json::ValueBuilder(userver::formats::json::Type::kObject);
    js_obj["package"]       = package_res;
    return js_obj.ExtractValue();
}

}  // namespace service::pg
