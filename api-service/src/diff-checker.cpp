#include "diff-checker.hpp"
#include "alpm_handle.hpp"

#include <alpm.h>

#include <algorithm>    // for for_each, all_of
#include <array>        // for array
#include <execution>    // for execution::unseq
#include <filesystem>   // for exist, copy_file
#include <ranges>       // for ranges
#include <string_view>  // for string_view

#include <userver/logging/log.hpp>

namespace fs = std::filesystem;

namespace {

// NOTE: REMOVE ME
using namespace service::alpm;

template <std::size_t N, const std::array<const char*, N>& db_names, const std::array<const char*, N>& db_servers>
auto init_alpm(std::string_view pacman_path, bool fetch_dbs) noexcept -> alpm_handle_t* {
    alpm_errno_t err{};
    alpm_handle_t* handle = alpm_initialize(pacman_path.data(), pacman_path.data(), &err);
    if (handle == nullptr) {
        return nullptr;
    }

    std::array<alpm_db_t*, db_names.size()> dbs{};
    for (std::size_t i = 0; i < db_names.size(); ++i) {
        dbs[i] = alpm_register_syncdb(handle, db_names[i], 0);
        alpm_db_add_server(dbs[i], db_servers[i]);
    }

    if (!fetch_dbs) {
        return handle;
    }

    auto ret = alpm_db_update(handle, alpm_get_syncdbs(handle), false);
    if (ret < 0) {
        LOG_ERROR("Failed to update DB! {}", alpm_strerror(alpm_errno(handle)));
        alpm_release(handle);
        return nullptr;
    }

    return handle;
}

auto init_alpm_arch_local_copy(std::string_view pacman_path) noexcept -> alpm_handle_t* {
    static constexpr std::array DB_NAMES{"core", "extra"};
    static constexpr std::array DB_SERVERS{"file:///var/lib/api-service/archrepo/", "file:///var/lib/api-service/archrepo/"};

    return init_alpm<DB_NAMES.size(), DB_NAMES, DB_SERVERS>(pacman_path, true);
}

auto init_alpm_arch(std::string_view pacman_path) noexcept -> alpm_handle_t* {
    static constexpr std::array DB_NAMES{"core", "extra"};
    static constexpr std::array DB_SERVERS{"https://geo.mirror.pkgbuild.com/core/os/x86_64/", "https://geo.mirror.pkgbuild.com/extra/os/x86_64/"};

    return init_alpm<DB_NAMES.size(), DB_NAMES, DB_SERVERS>(pacman_path, true);
}

auto get_pkgcache(const AlpmHandle& handle) noexcept -> AlpmList<AlpmPackage> {
    auto dbs = handle.syncdbs();
    if (dbs.empty()) {
        return AlpmList<AlpmPackage>{nullptr};
    }

    std::array<AlpmList<AlpmPackage>, 2> lists{};
    for (std::size_t i = 0; i < lists.size(); ++i) {
        auto db  = *dbs.get_nth(i);
        lists[i] = db.pkgcache();
    }
    return lists[0].join(lists[1]);
}

auto create_dirs_if_needed(std::string_view pacman_path) noexcept -> bool {
    try {
        if (!fs::exists(pacman_path)) {
            fs::create_directories(pacman_path);
        }
    } catch (const fs::filesystem_error& ex) {
        LOG_ERROR() << "Failed to create needed dirs" << ex;
        return false;
    }
    return true;
}

}  // namespace

namespace service::alpm {

void DiffComponent::detect_new_pkgs(const AlpmPackage& pkg) noexcept {
    // skip pkg if exist in local db copy
    /* clang-format off */
    if (local_head_handle_.is_package_available(pkg.name())) { return; }
    /* clang-format on */

    auto res_meta = convertmeta_from_alpm(pkg);
    auto res_deps = convertdeps_from_alpm(pkg);
    LOG_DEBUG("'{}/{}-{}-{}'", pkg.db().name(), pkg.name(), pkg.version(), pkg.arch());
    // insert_or_update_package(pkg.db().name(), pkg.name(), pkg.version(), pkg.filename(), res_meta, res_deps);
}

void DiffComponent::detect_diff_pkgs(const AlpmPackage& local_pkg) noexcept {
    auto pkgname = local_pkg.name();
    auto pkgver  = local_pkg.version();
    auto pkgarch = local_pkg.arch();

    /* clang-format off */
    if (pkgver.empty() || pkgname.empty() || pkgarch.empty()) { return; }
    /* clang-format on */

    // packages removed from upstream, but present in local copy
    auto arch_pkg = upstream_handle_.find_package(pkgname);
    if (!arch_pkg.has_value()) {
        LOG_DEBUG("stl: '{}/{}'", local_pkg.db().name(), pkgname);
        // remove_package(local_pkg.db().name(), pkgname);
        return;
    }

    // skip existing version of package
    if (arch_pkg->version() == pkgver) {
        return;
    }

    // version in upstream higher
    if (auto verres = alpm_pkg_vercmp(arch_pkg->version().data(), pkgver.data()); verres == 1) {
        auto res_meta = convertmeta_from_alpm(*arch_pkg);
        auto res_deps = convertdeps_from_alpm(*arch_pkg);
        LOG_DEBUG("old: '{}/{}'", arch_pkg->db().name(), arch_pkg->name());
        // insert_or_update_package(arch_pkg.db().name(), arch_pkg->name(), arch_pkg->version(), arch_pkg->filename(), res_meta, res_deps);
        return;
    }
}

void DiffComponent::run_check() noexcept {
    auto pkgcache = get_pkgcache(local_head_handle_);
    std::for_each(std::execution::unseq, pkgcache.begin(), pkgcache.end(),
        [&](auto local_pkg) {
            detect_diff_pkgs(local_pkg);
        });

    // detect new packages
    for (auto db : upstream_handle_.syncdbs()) {
        auto db_packagecache = db.pkgcache();

        std::for_each(std::execution::unseq, db_packagecache.begin(), db_packagecache.end(),
            [&](auto pkg) {
                detect_new_pkgs(pkg);
            });
    }

    // NOTE: manually cleanup due to double free
    local_head_handle_.release();
    upstream_handle_.release();
}

auto DiffComponent::init_handles() noexcept -> bool {
    // init required dirs
    auto dirs_func = [](auto&& pacman_path) {
        return create_dirs_if_needed(pacman_path);
    };
    if (!std::ranges::all_of(std::array{pacman_local_path_, pacman_upstream_path_}, std::move(dirs_func))) {
        return false;
    }

    // try to init local handle
    auto* alpm_local_handle = init_alpm_arch_local_copy(pacman_local_path_);
    if (!alpm_local_handle) {
        auto coredb_path  = fs::path{pacman_local_path_} / "core.db";
        auto extradb_path = fs::path{pacman_local_path_} / "extra.db";
        // we have both of them, but fails to populate it
        if (fs::exists(coredb_path) && fs::exists(extradb_path)) {
            LOG_ERROR() << "Failed to populate ALPM handle for local copy";
            return false;
        }
        // we don't have dbs fetched for the local copy yet
        if (!fs::exists(coredb_path) && !fs::exists(extradb_path)) {
            LOG_INFO() << "Local copy isn't fetched yet";
        }
    }
    local_head_handle_.reset(alpm_local_handle);

    // required to for upstream(remote copy) db to always initialize
    auto* alpm_arch_handle = init_alpm_arch(pacman_upstream_path_);
    upstream_handle_.reset(alpm_arch_handle);
    return alpm_arch_handle != nullptr;
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
