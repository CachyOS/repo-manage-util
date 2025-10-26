// Copyright (C) 2025 Vladislav Nepogodin
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along
// with this program; if not, write to the Free Software Foundation, Inc.,
// 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
#include "diff_checker.hpp"
#include "detect_gateway.hpp"
#include "queries.hpp"

#include <filesystem>   // for exists, copy_file
#include <string_view>  // for string_view

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
// #pragma GCC diagnostic ignored "-Wold-style-cast"
#endif

#include <userver/storages/postgres/cluster.hpp>
#include <userver/storages/postgres/io/uuid.hpp>
#include <userver/utils/boost_uuid4.hpp>

#include <boost/uuid/uuid.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace fs = std::filesystem;

namespace {

constexpr auto convert_from_ruststr(::rust::Str orig_str) noexcept -> std::string_view {
    return {orig_str.data(), orig_str.size()};
}

}  // namespace

namespace service::alpm {

DiffComponent::DiffComponent(userver::storages::postgres::ClusterPtr pg_cluster)
  : pg_cluster_(std::move(pg_cluster)) {
    auto gateway   = std::make_shared<service::rustlib::PgDetectGateway>(pg_cluster_);
    auto diff_comp = DiffComponentRust::init_diff_component(std::move(gateway));
    if (!diff_comp) {
        throw std::runtime_error("Failed to init diff component");
    }
    diff_component_ = std::make_unique<DiffComponentRust>(std::move(*diff_comp));
}

void DiffComponent::run_check() noexcept {
    diff_component_->run_check();
}

auto DiffComponent::refresh_handles() noexcept -> bool {
    return diff_component_->refresh_handles();
}

auto DiffComponent::update_local_copy() noexcept -> bool {
    // lets extra check if upstream dbs still preset on disk
    auto coredb_path  = fs::path{pacman_upstream_path_} / "sync" / "core.db";
    auto extradb_path = fs::path{pacman_upstream_path_} / "sync" / "extra.db";
    if (!fs::exists(coredb_path) && fs::exists(extradb_path)) {
        LOG_ERROR() << "Upstream DB copy got lost";
        return false;
    }

    auto coredb_destpath  = fs::path{pacman_local_path_} / "core.db";
    auto extradb_destpath = fs::path{pacman_local_path_} / "extra.db";
    try {
        const auto copy_options = fs::copy_options::overwrite_existing;
        fs::copy_file(coredb_path, coredb_destpath, copy_options);
        fs::copy_file(extradb_path, extradb_destpath, copy_options);
    } catch (const fs::filesystem_error& ex) {
        LOG_ERROR() << "Upstream DB copy got lost" << ex;
    }
    return true;
}

}  // namespace service::alpm

namespace service::rustlib {

auto PgDetectGateway::insert_or_update_repository(std::string_view repo_name) const -> bool {
    // skip if not configured to use cluster
    // clang-format off
    if (!pg_cluster_) { return true; }
    // clang-format on

    try {
        using userver::storages::postgres::ClusterHostType;
        auto result = pg_cluster_->Execute(ClusterHostType::kMaster, pg::query::kInsertRepository, repo_name);
        if (result.IsEmpty()) {
            // wasn't inserted
            LOG_DEBUG("Repository '{}' wasn't ins/upd", repo_name);
            return false;
        }
        auto uuid = result[0][0].As<boost::uuids::uuid>();
        return true;
    } catch (const std::exception& ex) {
        LOG_ERROR() << "Failed to ins/upd repo" << ex;
    }
    return false;
}

auto PgDetectGateway::insert_or_update_package(std::string_view repo_name, std::string_view pkg_name, std::string_view pkg_ver,
    std::string_view pkg_filename, const pg::utils::PackageMetadata& meta, const pg::utils::PackageDependencies& deps) const -> bool {
    LOG_DEBUG("old: '{}/{}'", repo_name, pkg_name);
    // skip if not configured to use cluster
    // clang-format off
    if (!pg_cluster_) { return true; }
    // clang-format on

    // insert/update repo before running
    if (!insert_or_update_repository(repo_name)) {
        return false;
    }

    try {
        using userver::storages::postgres::ClusterHostType;
        auto result = pg_cluster_->Execute(ClusterHostType::kMaster, pg::query::kInsertPackage, repo_name, pkg_name, pkg_ver, pkg_filename, meta, deps);
        if (result.IsEmpty()) {
            // wasn't removed
            LOG_DEBUG("Package '{}/{}' wasn't ins/upd", repo_name, pkg_name);
            return false;
        }
        return true;
    } catch (const std::exception& ex) {
        LOG_ERROR() << "Failed to ins/upd package" << ex;
    }
    return false;
}

auto PgDetectGateway::remove_package(std::string_view repo_name, std::string_view pkg_name) const -> bool {
    LOG_DEBUG("stl: '{}/{}'", repo_name, pkg_name);

    // skip if not configured to use cluster
    // clang-format off
    if (!pg_cluster_) { return true; }
    // clang-format on

    try {
        using userver::storages::postgres::ClusterHostType;
        auto result = pg_cluster_->Execute(ClusterHostType::kMaster, pg::query::kRemovePackage, repo_name, pkg_name);
        if (result.IsEmpty()) {
            // wasn't removed
            LOG_DEBUG("Package '{}/{}' wasn't removed", repo_name, pkg_name);
            return false;
        }
        return result[0][0].As<bool>();
    } catch (const std::exception& ex) {
        LOG_ERROR() << "Failed to remove package" << ex;
    }
    return false;
}

auto PgDetectGateway::insert_or_update_package_rust(::rust::Str repo_name, ::rust::Str pkg_name, ::rust::Str pkg_ver, ::rust::Str pkg_filename, const PackageMetadata& pkg_meta, const PackageDependencies& pkg_deps) const -> bool {
    return insert_or_update_package(convert_from_ruststr(repo_name), convert_from_ruststr(pkg_name), convert_from_ruststr(pkg_ver), convert_from_ruststr(pkg_filename), alpm::convertmeta_from_rust(pkg_meta), alpm::convertdeps_from_rust(pkg_deps));
}

auto PgDetectGateway::remove_package_rust(::rust::Str repo_name, ::rust::Str pkg_name) const -> bool {
    return remove_package(convert_from_ruststr(repo_name), convert_from_ruststr(pkg_name));
}

void log_error_msg(const std::string& message) noexcept {
    LOG_ERROR() << message;
}

void log_info_msg(const std::string& message) noexcept {
    LOG_INFO() << message;
}

}  // namespace service::rustlib
