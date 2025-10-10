#include "arch_repo_checker.hpp"
#include "package_files_handler.hpp"
#include "package_handler.hpp"
#include "packages_search_handler.hpp"
#include "split_package_handler.hpp"

#include <string>       // for string
#include <string_view>  // for string_view

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wc++23-attribute-extensions"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wc++23-attribute-extensions"
#endif

#include <pqxx/pqxx>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

#include <fmt/format.h>
#include <spdlog/spdlog.h>

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

#include <service/sql_queries.hpp>

#include <userver/clients/dns/component.hpp>
#include <userver/clients/http/component.hpp>
#include <userver/components/common_component_list.hpp>
#include <userver/components/common_server_component_list.hpp>
#include <userver/components/component.hpp>
#include <userver/components/component_list.hpp>
#include <userver/components/logging_configurator.hpp>
#include <userver/components/minimal_server_component_list.hpp>
#include <userver/congestion_control/component.hpp>
#include <userver/server/handlers/http_handler_base.hpp>
#include <userver/server/handlers/ping.hpp>
#include <userver/server/handlers/tests_control.hpp>
#include <userver/storages/postgres/cluster.hpp>
#include <userver/storages/postgres/component.hpp>
#include <userver/testsuite/testsuite_support.hpp>
#include <userver/utils/daemon_run.hpp>

#include <userver/formats/parse/to.hpp>
#include <userver/formats/yaml.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

static auto create_service_component_list() noexcept
    -> userver::components::ComponentList {
    return userver::components::MinimalServerComponentList()
        .Append<service::pg::PackageHandler>()
        .Append<service::pg::PackageFilesHandler>()
        .Append<service::pg::PackagesSearchHandler>()
        .Append<service::pg::SplitPackageHandler>()
        .Append<service::alpm::ArchRepoCheckerComponent>()
        .Append<userver::congestion_control::Component>()
        .Append<userver::components::Postgres>("repomanage-postgres-db-1")
        /* needed for testsuite */
        .Append<userver::server::handlers::Ping>()
        .Append<userver::components::HttpClient>()
        .Append<userver::components::TestsuiteSupport>()
        .Append<userver::server::handlers::TestsControl>()
        /* needed for testsuite */
        .Append<userver::clients::dns::Component>()
        .Append<userver::components::LoggingConfigurator>();
}

static auto create_table(std::string_view config_file_path, std::string_view config_vars_path) noexcept -> bool {
    auto config_file_yml = userver::formats::yaml::blocking::FromFile(config_file_path.data());
    auto connection_url  = config_file_yml["components_manager"]["components"]
                                         ["repomanage-postgres-db-1"]["dbconnection"]
                                             .As<std::string>();

    std::string final_path_to_confvars{config_vars_path};
    if (config_vars_path.empty()) {
        if (auto conf_var = config_file_yml["config_vars"]; conf_var.IsString()) {
            final_path_to_confvars = conf_var.As<std::string>();
        }
    }

    auto config_vars_yml = userver::formats::yaml::blocking::FromFile(final_path_to_confvars);

    // we handle that at setup step with testsuite
    if (config_vars_yml["testsuite-enabled"].As<bool>()) {
        return true;
    }

    if (connection_url.starts_with('$')) {
        auto conn_var_name = std::string_view{connection_url}.substr(1);
        if (final_path_to_confvars.empty()) {
            fmt::print("requires '{}' variable to be declared in config_vars\n", conn_var_name);
            return false;
        }
        connection_url = config_vars_yml[conn_var_name].As<std::string>();
    }

    try {
        auto connection = pqxx::connection(connection_url.data());
        if (!connection.is_open()) {
            spdlog::error("Failed to connect to the database.");
            return false;
        }
        spdlog::trace("Finished initiating PostgreSQL DB connection");

        pqxx::work txn(connection);
        txn.exec(std::string{service::sql::kInitDb.GetStatementView()});
        txn.commit();
    } catch (const std::exception& ex) {
        spdlog::error("Failed to connect to the database. what='{}'", ex.what());
        return false;
    }

    return true;
}

int main(int argc, char** argv) {
    using namespace std::string_view_literals;

    std::string_view config_file_path{"postgres_service.yaml"sv};
    std::string_view config_vars_path{};
    bool show_help{};
    for (int i = 1; i < argc; ++i) {
        auto arg = std::string_view{argv[i]};
        if (arg == "-c"sv || arg == "--config"sv) {
            if ((i + 1) < argc) {
                config_file_path = argv[i + 1];
                ++i;
                continue;
            }
        }

        if (arg == "--config_vars"sv) {
            if ((i + 1) < argc) {
                config_vars_path = argv[i + 1];
                ++i;
                continue;
            }
        }

        if (arg == "-h"sv || arg == "--help"sv) {
            show_help = true;
        }
    }
    if (!show_help && !create_table(config_file_path, config_vars_path)) {
        return 1;
    }

    const auto& service_component_list = create_service_component_list();
    return userver::utils::DaemonMain(argc, argv, service_component_list);
}
