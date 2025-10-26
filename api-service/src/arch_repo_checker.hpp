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

#include "diff_checker.hpp"

#include <chrono>       // for seconds
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

#include <userver/components/component.hpp>
#include <userver/concurrent/variable.hpp>
#include <userver/storages/postgres/cluster.hpp>
#include <userver/storages/postgres/component.hpp>
#include <userver/utils/periodic_task.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace service::alpm {

class ArchRepoCheckerComponent final : public userver::components::ComponentBase {
 public:
    // `kName` is used as the component name in static config
    static constexpr std::string_view kName = "arch-repo-checker";

    ArchRepoCheckerComponent(const userver::components::ComponentConfig& config,
        const userver::components::ComponentContext& component_context);

    static userver::yaml_config::Schema GetStaticConfigSchema();

 private:
    void StartUpdateTask();
    void RunUpdateTask() noexcept;

    const std::chrono::seconds update_period_;
    userver::engine::TaskProcessor& blocking_task_processor_;
    userver::utils::PeriodicTask update_task_;
    userver::storages::postgres::ClusterPtr pg_cluster_;

    userver::concurrent::Variable<DiffComponent> diff_component_;
};

}  // namespace service::alpm

template <>
inline constexpr bool userver::components::kHasValidate<service::alpm::ArchRepoCheckerComponent> = true;
