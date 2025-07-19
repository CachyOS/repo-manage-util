#include "split_package_handler.hpp"
#include "http_utils.hpp"
#include "package_utils.hpp"
#include "queries.hpp"

#include <string>   // for string
#include <utility>  // for move
#include <vector>   // for vector

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

#include <userver/server/handlers/exceptions.hpp>
#include <userver/storages/postgres/cluster.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace {
using PackageRow = service::pg::utils::BriefPackageRow;
}  // namespace

namespace service::pg {

userver::formats::json::Value SplitPackageHandler::HandleRequestJsonThrow(
    const userver::server::http::HttpRequest& request,
    const userver::formats::json::Value&,
    userver::server::request::RequestContext&) const {
    static constexpr auto kNotFound = "Package not found!";

    // set http headers
    http::utils::set_response_http_headers(request);

    // get args
    const auto& repo    = request.GetPathArg("repo");
    const auto& pkgbase = request.GetPathArg("pkgbase");

    using userver::storages::postgres::ClusterHostType;
    auto result = pg_cluster_->Execute(ClusterHostType::kSlave, query::kSelectSplitPackage, repo, pkgbase);
    if (result.IsEmpty()) {
        throw userver::server::handlers::ResourceNotFound(
            userver::server::handlers::ExternalBody{kNotFound});
    }

    auto js_array = userver::formats::json::ValueBuilder(userver::formats::json::Type::kArray);
    for (auto row : result.AsSetOf<PackageRow>(userver::storages::postgres::kRowTag)) {
        static_assert(std::is_same_v<decltype(row), PackageRow>,
            "Iterate over aggregate classes");

        js_array.PushBack(std::move(row));
    }

    return js_array.ExtractValue();
}

}  // namespace service::pg
