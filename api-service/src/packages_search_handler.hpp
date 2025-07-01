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
// #pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/components/component.hpp>
#include <userver/server/handlers/http_handler_json_base.hpp>
#include <userver/storages/postgres/cluster.hpp>
#include <userver/storages/postgres/component.hpp>

#include <userver/formats/json.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::pg {

class PackagesSearchHandler final : public userver::server::handlers::HttpHandlerJsonBase {
 public:
    // `kName` is used as the component name in static config
    static constexpr std::string_view kName = "handler-packages-search";

    PackagesSearchHandler(const userver::components::ComponentConfig& config,
        const userver::components::ComponentContext& component_context)
      : HttpHandlerJsonBase(config, component_context),
        pg_cluster_(
            component_context
                .FindComponent<userver::components::Postgres>("repomanage-postgres-db-1")
                .GetCluster()) { }

    userver::formats::json::Value HandleRequestJsonThrow(
        const userver::server::http::HttpRequest&,
        const userver::formats::json::Value&,
        userver::server::request::RequestContext& ctx) const override;

 private:
    userver::storages::postgres::ClusterPtr pg_cluster_;
};

}  // namespace service::pg
