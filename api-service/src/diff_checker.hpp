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

#include "diff_component_rust.hpp"

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
#include <userver/storages/postgres/component.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace service::alpm {

class DiffComponent final {
 public:
    explicit DiffComponent(userver::storages::postgres::ClusterPtr pg_cluster);

    void run_check() noexcept;
    auto refresh_handles() noexcept -> bool;
    auto update_local_copy() noexcept -> bool;

 private:
    const std::string_view pacman_local_path_    = "/var/lib/api-service/archrepo";
    const std::string_view pacman_upstream_path_ = "/tmp/pacman_upstream_arch";

    DiffComponentRustPtr diff_component_;
    userver::storages::postgres::ClusterPtr pg_cluster_;
};

}  // namespace service::alpm
