#include "package_handler.hpp"
#include "packages_search_handler.hpp"

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

#include <userver/clients/dns/component.hpp>
#include <userver/clients/http/component.hpp>
#include <userver/components/common_component_list.hpp>
#include <userver/components/common_server_component_list.hpp>
#include <userver/components/component.hpp>
#include <userver/components/component_list.hpp>
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
        .Append<service::pg::PackagesSearchHandler>()
        .Append<userver::congestion_control::Component>()
        .Append<userver::components::Postgres>("repomanage-postgres-db-1")
        /* needed for testsuite */
        .Append<userver::server::handlers::Ping>()
        .Append<userver::components::HttpClient>()
        .Append<userver::components::TestsuiteSupport>()
        .Append<userver::server::handlers::TestsControl>()
        /* needed for testsuite */
        .Append<userver::clients::dns::Component>();
}

static auto create_table(std::string_view config_file_path, std::string_view config_vars_path) noexcept -> bool {
    static constexpr auto kCreateTable = R"~(
CREATE SCHEMA IF NOT EXISTS helper_schema;

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE IF NOT EXISTS repositories (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_name  TEXT UNIQUE NOT NULL,
    repo_desc  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS packages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_name       TEXT NOT NULL REFERENCES repositories(repo_name) ON DELETE CASCADE ON UPDATE CASCADE,
    pkg_name        TEXT NOT NULL,
    pkg_version     TEXT NOT NULL,
    pkg_filename    TEXT NOT NULL,
    pkg_base        TEXT,
    pkg_desc        TEXT,
    pkg_groups      TEXT[],
    pkg_url         TEXT,
    pkg_license     TEXT[],
    pkg_arch        TEXT,
    pkg_builddate   TIMESTAMPTZ,
    pkg_packager    TEXT,
    pkg_csize       BIGINT,
    pkg_isize       BIGINT,
    pkg_sha256sum   TEXT,
    pkg_pgpsig      TEXT,
    pkg_replaces    TEXT[],
    pkg_depends     TEXT[],
    pkg_optdepends  TEXT[],
    pkg_makedepends TEXT[],
    pkg_checkdepends TEXT[],
    pkg_conflicts   TEXT[],
    pkg_provides    TEXT[],
    pkg_files       TEXT[],
    updated         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- The natural key for a package instance.
    CONSTRAINT uniq_pkg UNIQUE(repo_name, pkg_name)
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_pkg_idx ON packages (repo_name, pkg_name);

CREATE INDEX IF NOT EXISTS idx_packages_name ON packages(pkg_name);
CREATE INDEX IF NOT EXISTS idx_packages_filename ON packages(pkg_filename);
CREATE INDEX IF NOT EXISTS idx_packages_arch ON packages(pkg_arch);

CREATE INDEX IF NOT EXISTS idx_packages_search ON packages USING gin (pkg_name gin_trgm_ops, pkg_desc gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_packages_depends ON packages USING gin (pkg_depends);
CREATE INDEX IF NOT EXISTS idx_packages_provides ON packages USING gin (pkg_provides);
CREATE INDEX IF NOT EXISTS idx_packages_files ON packages USING gin (pkg_files);

DO $$
BEGIN
IF NOT EXISTS (select 1 from pg_type where typname = 'public_package') then
CREATE TYPE helper_schema.public_package AS (
  repo_name       TEXT,
  pkg_name        TEXT,
  pkg_version     TEXT,
  pkg_base        TEXT,
  pkg_desc        TEXT,
  pkg_groups      TEXT[],
  pkg_url         TEXT,
  pkg_license     TEXT[],
  pkg_arch        TEXT,
  pkg_builddate   INTEGER,
  pkg_packager    TEXT,
  pkg_csize       BIGINT,
  pkg_isize       BIGINT,
  pkg_sha256sum   TEXT,
  pkg_pgpsig      TEXT,
  pkg_replaces    TEXT[],
  pkg_depends     TEXT[],
  pkg_optdepends  TEXT[],
  pkg_makedepends TEXT[],
  pkg_checkdepends TEXT[],
  pkg_conflicts   TEXT[],
  pkg_provides    TEXT[],
  pkg_files       TEXT[],
  updated         INTEGER
);
END IF;
END $$;

DO $$
BEGIN
IF NOT EXISTS (select 1 from pg_type where typname = 'brief_package') then
CREATE TYPE helper_schema.brief_package AS (
  pkg_name TEXT,
  repo_name TEXT,
  pkg_arch TEXT,
  pkg_version TEXT,
  pkg_desc TEXT,
  pkg_builddate INTEGER
);
ELSE
ALTER TYPE helper_schema.brief_package
RENAME ATTRIBUTE updated TO pkg_builddate;
END IF;
END $$;

DO $$
BEGIN
IF NOT EXISTS (select 1 from pg_type where typname = 'brief_package_page_result') then
CREATE TYPE helper_schema.brief_package_page_result AS (
  total_packages INTEGER,
  packages helper_schema.brief_package[]
);
END IF;
END $$;

CREATE OR REPLACE FUNCTION helper_schema.get_package(_repo_name text, _pkg_arch text, _pkg_name text)
        RETURNS SETOF helper_schema.public_package
        AS $$
BEGIN
        RETURN QUERY
        SELECT p.repo_name,
               p.pkg_name,
               p.pkg_version,
               p.pkg_base,
               p.pkg_desc,
               p.pkg_groups,
               p.pkg_url,
               p.pkg_license,
               p.pkg_arch,
               extract(epoch from p.pkg_builddate)::INTEGER AS pkg_builddate,
               p.pkg_packager,
               p.pkg_csize,
               p.pkg_isize,
               p.pkg_sha256sum,
               p.pkg_pgpsig,
               p.pkg_replaces,
               p.pkg_depends,
               p.pkg_optdepends,
               p.pkg_makedepends,
               p.pkg_checkdepends,
               p.pkg_conflicts,
               p.pkg_provides,
               p.pkg_files,
               extract(epoch from p.updated)::INTEGER AS updated
        FROM packages p
        WHERE
            (p.repo_name = _repo_name) AND
            (p.pkg_arch = _pkg_arch) AND
            (p.pkg_name = _pkg_name);
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.get_brief_packages()
        RETURNS SETOF helper_schema.brief_package
        AS $$
BEGIN
        RETURN QUERY
        SELECT p.pkg_name,
               p.repo_name,
               p.pkg_arch,
               p.pkg_version,
               p.pkg_desc,
               extract(epoch from p.pkg_builddate)::INTEGER AS pkg_builddate
        FROM packages p;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.get_brief_packages_with_filters(_repo_filter text[] = null, _arch_filter text[] = null)
        RETURNS SETOF helper_schema.brief_package
        AS $$
BEGIN
        RETURN QUERY
        SELECT *
        FROM helper_schema.get_brief_packages() AS p
        WHERE
            (p.repo_name = ANY (_repo_filter) OR array_length(_repo_filter, 1) IS NULL) AND
            (p.pkg_arch = ANY (_arch_filter) OR array_length(_arch_filter, 1) IS NULL)
        ORDER BY
            p.pkg_name;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.search_packages_with_filters(_query text = null, _repo_filter text[] = null, _arch_filter text[] = null)
        RETURNS SETOF helper_schema.brief_package
        AS $$
BEGIN
        RETURN QUERY
        SELECT *
        FROM helper_schema.get_brief_packages_with_filters(_repo_filter, _arch_filter) AS p
        WHERE
            _query IS NULL OR
            (p.pkg_name ILIKE '%' || _query || '%' OR
            p.pkg_desc ILIKE '%' || _query || '%');
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.search_offset_packages_with_filters(_limit integer = 100, _offset integer = 0, _query text = null, _repo_filter text[] = null, _arch_filter text[] = null)
        RETURNS SETOF helper_schema.brief_package
        AS $$
BEGIN
        RETURN QUERY
        SELECT *
        FROM helper_schema.search_packages_with_filters(_query, _repo_filter, _arch_filter) AS p
        LIMIT _limit OFFSET _offset;
END;
$$
LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION helper_schema.get_page_search_packages_with_offset(_limit integer = 100, _offset integer = 0, _query text = null, _repo_filter text[] = null, _arch_filter text[] = null)
        RETURNS helper_schema.brief_package_page_result
        AS $$
DECLARE
        _result helper_schema.brief_package_page_result;
BEGIN
        SELECT COUNT(*) INTO _result.total_packages FROM helper_schema.search_packages_with_filters(_query, _repo_filter, _arch_filter);

        WITH paginated_data AS (
            SELECT *
            FROM helper_schema.search_offset_packages_with_filters(_limit, _offset, _query, _repo_filter, _arch_filter)
        )
        SELECT
            COALESCE(
                array_agg(
                    ROW(pd.*)::helper_schema.brief_package
                ),
            ARRAY[]::helper_schema.brief_package[]) INTO _result.packages
        FROM paginated_data pd;

        RETURN _result;
END;
$$
LANGUAGE plpgsql STABLE;
    )~";

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
        txn.exec(kCreateTable);
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
