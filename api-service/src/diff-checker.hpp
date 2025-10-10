#pragma once

#include "alpm_handle.hpp"

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
    DiffComponent() = default;

    void run_check() noexcept;
    auto init_handles() noexcept -> bool;
    auto update_local_copy() noexcept -> bool;

 private:
    void detect_new_pkgs(const AlpmPackage& pkg) noexcept;
    void detect_diff_pkgs(const AlpmPackage& local_pkg) noexcept;

    const std::string_view pacman_local_path_    = "/var/lib/api-service/archrepo";
    const std::string_view pacman_upstream_path_ = "/tmp/pacman_upstream_arch";

    AlpmHandle local_head_handle_;
    AlpmHandle upstream_handle_;
    userver::storages::postgres::ClusterPtr pg_cluster_;
};

}  // namespace service::alpm
