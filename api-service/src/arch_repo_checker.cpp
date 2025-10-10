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
#include "arch_repo_checker.hpp"

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
#include <userver/utils/periodic_task.hpp>
#include <userver/yaml_config/merge_schemas.hpp>

#ifdef __clang__
#pragma clang diagnostic pop
#elifdef __GNUC__
#pragma GCC diagnostic pop
#endif

namespace service::alpm {

ArchRepoCheckerComponent::ArchRepoCheckerComponent(const userver::components::ComponentConfig& component_config, const userver::components::ComponentContext& component_context)
  : ComponentBase(component_config, component_context),
    update_period_(component_config["update-period"].As<std::chrono::seconds>()),
    blocking_task_processor_(component_context.GetTaskProcessor(component_config["blocking_task_processor"].As<std::string>())),
    pg_cluster_(
        component_context
            .FindComponent<userver::components::Postgres>("repomanage-postgres-db-1")
            .GetCluster()),
    diff_component_(pg_cluster_) {
    // start periodic task
    StartUpdateTask();
}

void ArchRepoCheckerComponent::StartUpdateTask() {
    LOG_INFO() << "Start task for archrepo periodic updates";
    const userver::utils::PeriodicTask::Settings periodic_settings{update_period_};
    update_task_.Start("archrepo_update", periodic_settings, [this]() {
        try {
            auto data_ptr = diff_component_.Lock();
            if (!data_ptr->refresh_handles()) {
                LOG_ERROR() << "Failed to init ALPM handles";
                return;
            }

            data_ptr->run_check();
            if (!data_ptr->update_local_copy()) {
                LOG_ERROR() << "Failed to update local db copy";
            }
        } catch (const std::exception& ex) {
            LOG_ERROR() << "ArchRepo checker failed: " << ex;
        }
    });
}

userver::yaml_config::Schema ArchRepoCheckerComponent::GetStaticConfigSchema() {
    return userver::yaml_config::MergeSchemas<userver::components::ComponentBase>(R"(
    # yaml
    type: object
    description: |
      Component for pulling archlinux repo information on-demand
    additionalProperties: false
    properties:
      blocking_task_processor:
          type: string
          description: (*required*) task processor to run periodic task
      update-period:
          type: string
          description: (*required*) interval between Update invocations
  )");
}

}  // namespace service::alpm
