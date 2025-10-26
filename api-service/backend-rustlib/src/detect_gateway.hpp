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

#pragma once

#include <memory>
#include <utility>
#include <string_view>

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wold-style-cast"
#pragma clang diagnostic ignored "-Wdollar-in-identifier-extension"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdeprecated-this-capture"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wnull-dereference"
#pragma GCC diagnostic ignored "-Wuseless-cast"
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsuggest-final-types"
#pragma GCC diagnostic ignored "-Wsuggest-attribute=pure"
#pragma GCC diagnostic ignored "-Wconversion"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#endif

#include "rust/cxx.h"

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace userver::storages::postgres {
class Cluster;
using ClusterPtr = std::shared_ptr<Cluster>;
}  // namespace userver::storages::postgres

namespace service::pg::utils {
struct PackageMetadata;
struct PackageDependencies;
}  // namespace service::pg::utils

namespace service::rustlib {
struct PackageMetadata;
struct PackageDependencies;
class PgDetectGateway {
 public:
    explicit PgDetectGateway(userver::storages::postgres::ClusterPtr pg_cluster)
      : pg_cluster_(std::move(pg_cluster)) { }

    auto remove_package_rust(::rust::Str repo_name, ::rust::Str pkg_name) const -> bool;
    auto insert_or_update_package_rust(::rust::Str, ::rust::Str, ::rust::Str, ::rust::Str, PackageMetadata const&, PackageDependencies const&) const -> bool;

 private:
    auto insert_or_update_repository(std::string_view repo_name) const -> bool;
    auto insert_or_update_package(std::string_view repo_name, std::string_view pkg_name, std::string_view pkg_ver,
        std::string_view pkg_filename, const pg::utils::PackageMetadata& meta, const pg::utils::PackageDependencies& deps) const -> bool;
    auto remove_package(std::string_view repo_name, std::string_view pkg_name) const -> bool;

    userver::storages::postgres::ClusterPtr pg_cluster_;
};

using PgDetectGatewayPtr = std::shared_ptr<PgDetectGateway>;

void log_error_msg(const std::string& message) noexcept;
void log_info_msg(const std::string& message) noexcept;

}  // namespace service::rustlib
