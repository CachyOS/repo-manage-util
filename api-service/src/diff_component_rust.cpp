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
#include "diff_component_rust.hpp"

#include <algorithm>  // for transform
#include <ranges>     // for ranges
#include <string>     // for string
#include <utility>    // for make_optional
#include <vector>     // for vector

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wold-style-cast"
#pragma clang diagnostic ignored "-Wdollar-in-identifier-extension"
#pragma clang diagnostic ignored "-Wsign-conversion"
#pragma clang diagnostic ignored "-Wdeprecated-this-capture"
#pragma clang diagnostic ignored "-Wshorten-64-to-32"
#pragma clang diagnostic ignored "-Wdouble-promotion"
#pragma clang diagnostic ignored "-Wdeprecated-literal-operator"
#pragma clang diagnostic ignored "-Wshadow"
#pragma clang diagnostic ignored "-Wimplicit-int-float-conversion"
#elifdef __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wnull-dereference"
#pragma GCC diagnostic ignored "-Wuseless-cast"
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsuggest-final-types"
#pragma GCC diagnostic ignored "-Wsuggest-attribute=pure"
#pragma GCC diagnostic ignored "-Wconversion"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#endif

#include "backend-rustlib-cxxbridge/lib.h"

#include <userver/logging/log.hpp>
#include <userver/storages/postgres/cluster.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace {

constexpr auto convert_rust_vec_string(auto&& rust_vec) noexcept -> std::vector<std::string> {
    std::vector<std::string> res_vec{};
    res_vec.reserve(rust_vec.size());
    std::ranges::transform(rust_vec, std::back_inserter(res_vec),
        [](auto&& elem) { return std::string{elem}; });
    return res_vec;
}

}  // namespace

namespace service::alpm {

auto DiffComponentRust::init_diff_component(service::rustlib::PgDetectGatewayPtr gateway) noexcept -> std::optional<DiffComponentRust> {
    try {
        auto&& component = DiffComponentRust(service::rustlib::init_diff_component(gateway));
        return std::make_optional<DiffComponentRust>(std::move(component));
    } catch (const std::exception& e) {
        LOG_ERROR("failed to init diff component: {}", e.what());
    }
    return std::nullopt;
}

auto DiffComponentRust::refresh_handles() noexcept -> bool {
    return m_component->refresh_handles();
}

void DiffComponentRust::run_check() noexcept {
    try {
        m_component->run_check();
    } catch (const std::exception& e) {
        LOG_ERROR("failed to run diff check: {}", e.what());
    }
}

service::pg::utils::PackageMetadata convertmeta_from_rust(const service::rustlib::PackageMetadata& pkg_meta) noexcept {
    return service::pg::utils::PackageMetadata{
        .pkg_base      = std::string{pkg_meta.pkg_base},
        .pkg_desc      = std::string{pkg_meta.pkg_desc},
        .pkg_groups    = convert_rust_vec_string(pkg_meta.pkg_groups),
        .pkg_url       = std::string{pkg_meta.pkg_url},
        .pkg_license   = convert_rust_vec_string(pkg_meta.pkg_license),
        .pkg_arch      = std::string{pkg_meta.pkg_arch},
        .pkg_builddate = userver::storages::postgres::TimePointTz{std::chrono::system_clock::from_time_t(pkg_meta.pkg_builddate)},
        .pkg_packager  = std::string{pkg_meta.pkg_packager},
        .pkg_csize     = pkg_meta.pkg_csize,
        .pkg_isize     = pkg_meta.pkg_isize,
        .pkg_sha256sum = std::string{pkg_meta.pkg_sha256sum},
        .pkg_pgpsig    = std::string{pkg_meta.pkg_pgpsig},
    };
}

pg::utils::PackageDependencies convertdeps_from_rust(const service::rustlib::PackageDependencies& pkg_deps) noexcept {
    return pg::utils::PackageDependencies{
        .pkg_replaces     = convert_rust_vec_string(pkg_deps.pkg_replaces),
        .pkg_depends      = convert_rust_vec_string(pkg_deps.pkg_depends),
        .pkg_optdepends   = convert_rust_vec_string(pkg_deps.pkg_optdepends),
        .pkg_makedepends  = convert_rust_vec_string(pkg_deps.pkg_makedepends),
        .pkg_checkdepends = convert_rust_vec_string(pkg_deps.pkg_checkdepends),
        .pkg_conflicts    = convert_rust_vec_string(pkg_deps.pkg_conflicts),
        .pkg_provides     = convert_rust_vec_string(pkg_deps.pkg_provides),
        .pkg_files        = {},
    };
}

}  // namespace service::alpm
