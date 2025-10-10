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

#include "package_utils.hpp"

#include <memory>
#include <optional>

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

#include <userver/storages/postgres/postgres_fwd.hpp>

#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

namespace service::rustlib {
struct DiffComponent;
struct PackageMetadata;
struct PackageDependencies;
class PgDetectGateway;
using PgDetectGatewayPtr = std::shared_ptr<PgDetectGateway>;
}  // namespace service::rustlib

namespace service::alpm {

class DiffComponentRust {
 public:
    /// @brief Initializes and configures ALPM handles for diff.
    static auto init_diff_component(::service::rustlib::PgDetectGatewayPtr gateway) noexcept -> std::optional<DiffComponentRust>;

    /// @brief Re-initializes the ALPM handles.
    /// @return True if the handles were 'refreshed', false otherwise.
    auto refresh_handles() noexcept -> bool;

    /// @brief Runs diff check on archlinux repos.
    void run_check() noexcept;

    // explicitly deleted
    DiffComponentRust() = delete;

 private:
    explicit DiffComponentRust(::rust::Box<::service::rustlib::DiffComponent>&& component)
      : m_component(std::move(component)) { }

    /// @brief A pointer to the AlpmManager structure from Rust code.
    ::rust::Box<::service::rustlib::DiffComponent> m_component;
};

/// @brief A shared pointer to a DiffComponentRust object.
using DiffComponentRustPtr = std::shared_ptr<DiffComponentRust>;

pg::utils::PackageMetadata convertmeta_from_rust(const ::service::rustlib::PackageMetadata& pkg_meta) noexcept;
pg::utils::PackageDependencies convertdeps_from_rust(const ::service::rustlib::PackageDependencies& pkg_deps) noexcept;

}  // namespace service::alpm
