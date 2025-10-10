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

use std::fs;
use std::path::Path;

use alpm::{Alpm, SigLevel};
use alpm_utils::DbListExt;
use anyhow::{Context, Result};

#[cxx::bridge(namespace = "service::rustlib")]
mod ffi {
    #[derive(Debug, Default, Clone, PartialEq)]
    pub struct PackageMetadata {
        pkg_base: String,
        pkg_desc: String,
        pkg_groups: Vec<String>,
        pkg_url: String,
        pkg_license: Vec<String>,
        pkg_arch: String,
        pkg_builddate: i64,
        pkg_packager: String,
        pkg_csize: i64,
        pkg_isize: i64,
        pkg_sha256sum: String,
        pkg_pgpsig: String,
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    pub struct PackageDependencies {
        pkg_replaces: Vec<String>,
        pkg_depends: Vec<String>,
        pkg_optdepends: Vec<String>,
        pkg_makedepends: Vec<String>,
        pkg_checkdepends: Vec<String>,
        pkg_conflicts: Vec<String>,
        pkg_provides: Vec<String>,
        pkg_files: Vec<String>,
    }

    unsafe extern "C++" {
        include!("detect_gateway.hpp");

        type PgDetectGateway;
        fn remove_package_rust(&self, repo_name: &str, pkgname: &str) -> bool;
        fn insert_or_update_package_rust(
            &self,
            repo_name: &str,
            pkgname: &str,
            pkg_ver: &str,
            pkg_filename: &str,
            meta: &PackageMetadata,
            deps: &PackageDependencies,
        ) -> bool;

        fn log_error_msg(message: &CxxString);
        fn log_info_msg(message: &CxxString);
    }

    extern "Rust" {
        type DiffComponent;

        fn init_diff_component(gateway: SharedPtr<PgDetectGateway>) -> Result<Box<DiffComponent>>;
        fn refresh_handles(&mut self) -> bool;
        fn run_check(&self) -> Result<()>;
    }
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    name: String,
    servers: Vec<String>,
}

fn init_diff_component(
    gateway: cxx::SharedPtr<ffi::PgDetectGateway>,
) -> Result<Box<DiffComponent>> {
    let component = DiffComponent::new(gateway).context("Failed to initialize diff component")?;
    Ok(Box::new(component))
}

pub struct AlpmManager {
    handle: Alpm,
    db_path: String,
    rootdir: String,
    fetch_dbs: bool,
    pacman_dbs: Vec<DbConfig>,
}

impl AlpmManager {
    pub fn new(
        db_path: String,
        rootdir: String,
        fetch_dbs: bool,
        pacman_dbs: Vec<DbConfig>,
    ) -> Result<Self> {
        let handle = init_alpm(&db_path, &rootdir, fetch_dbs, &pacman_dbs)?;
        Ok(AlpmManager { handle, db_path, rootdir, fetch_dbs, pacman_dbs })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let handle = init_alpm(&self.db_path, &self.rootdir, self.fetch_dbs, &self.pacman_dbs)?;
        self.handle = handle;
        Ok(())
    }

    pub fn is_package_available(&self, pkgname: &str) -> bool {
        self.find_package(pkgname).is_some()
    }

    pub fn find_package(&self, pkgname: &str) -> Option<&alpm::Package> {
        self.handle.syncdbs().pkg(pkgname).ok()
    }

    pub fn pkgcache(&self) -> Vec<&alpm::Package> {
        self.handle.syncdbs().iter().map(|db| db.pkgs()).flatten().collect()
    }
}

impl TryFrom<&alpm::Package> for ffi::PackageMetadata {
    type Error = ();

    fn try_from(pkg: &alpm::Package) -> Result<Self, Self::Error> {
        Ok(Self {
            pkg_base: pkg.base().map(|s| s.to_string()).ok_or(())?,
            pkg_desc: pkg.desc().map(|s| s.to_string()).ok_or(())?,
            pkg_groups: Some(pkg.groups().iter().map(|s| s.to_string()).collect()).ok_or(())?,
            pkg_url: pkg.url().map(|s| s.to_string()).ok_or(())?,
            pkg_license: Some(pkg.licenses().iter().map(|s| s.to_string()).collect()).ok_or(())?,
            pkg_arch: pkg.arch().map(|s| s.to_string()).ok_or(())?,
            pkg_builddate: pkg.build_date(),
            pkg_packager: pkg.packager().map(|s| s.to_string()).ok_or(())?,
            pkg_csize: pkg.size(),
            pkg_isize: pkg.isize(),
            pkg_sha256sum: pkg.sha256sum().map(|s| s.to_string()).ok_or(())?,
            // TODO(vnepogodin): should store keyid instead, for which need to get handle from
            // package
            pkg_pgpsig: pkg.base64_sig().map(|x| x.to_owned()).ok_or(())?,
        })
    }
}

impl From<&alpm::Package> for ffi::PackageDependencies {
    fn from(pkg: &alpm::Package) -> Self {
        Self {
            pkg_replaces: pkg.replaces().into_iter().map(|s| s.to_string()).collect(),
            pkg_depends: pkg.depends().into_iter().map(|s| s.to_string()).collect(),
            pkg_optdepends: pkg.optdepends().into_iter().map(|s| s.to_string()).collect(),
            pkg_makedepends: pkg.makedepends().into_iter().map(|s| s.to_string()).collect(),
            pkg_checkdepends: pkg.checkdepends().into_iter().map(|s| s.to_string()).collect(),
            pkg_conflicts: pkg.conflicts().into_iter().map(|s| s.to_string()).collect(),
            pkg_provides: pkg.provides().into_iter().map(|s| s.to_string()).collect(),
            pkg_files: pkg.files().files().iter().map(|s| s.name().to_string()).collect(),
        }
    }
}

// NOTE: hardcoding these paths for now
const PACMAN_LOCAL_PATH: &str = "/var/lib/api-service/archrepo";
const PACMAN_UPSTREAM_PATH: &str = "/tmp/pacman_upstream_arch";

pub struct DiffComponent {
    local_head_handle: Option<Box<AlpmManager>>,
    upstream_handle: Box<AlpmManager>,
    gateway: cxx::SharedPtr<ffi::PgDetectGateway>,
}

impl DiffComponent {
    pub fn new(gateway: cxx::SharedPtr<ffi::PgDetectGateway>) -> Result<Self> {
        // init required dirs
        fs::create_dir_all(PACMAN_LOCAL_PATH)
            .context("failed to create local archrepo directory")?;
        fs::create_dir_all(PACMAN_UPSTREAM_PATH)
            .context("failed to create upstream archrepo directory")?;

        // try to init local handle
        let local_head_handle = match init_alpm_arch_local_copy(PACMAN_LOCAL_PATH) {
            Ok(handle) => Some(Box::new(handle)),
            Err(e) => {
                let local_path = Path::new(PACMAN_LOCAL_PATH);
                let coredb_path = local_path.join("core.db");
                let extradb_path = local_path.join("extra.db");

                // we have both of them, but fails to populate it
                if coredb_path.exists() && extradb_path.exists() {
                    return Err(e).context("Failed to populate ALPM handle for local copy");
                }

                // we don't have dbs fetched for the local copy yet
                if !coredb_path.exists() && !extradb_path.exists() {
                    // NOTE: dirty, but doesn't need to hook tracing/log to userver logging system
                    log_info_msg("Local copy isn't fetched yet".into());
                    None
                } else {
                    anyhow::bail!("Inconsistent state for local DBs");
                }
            },
        };

        // required to for upstream(remote copy) db to always initialize
        let upstream_handle = Box::new(
            init_alpm_arch(PACMAN_UPSTREAM_PATH)
                .context("Failed to populate upstream ALPM handle")?,
        );

        Ok(DiffComponent { local_head_handle, upstream_handle, gateway })
    }

    pub fn refresh_handles(&mut self) -> bool {
        if let Some(handle) = self.local_head_handle.as_mut() {
            handle.refresh().is_ok() && self.upstream_handle.refresh().is_ok()
        } else {
            let local_head_handle = init_alpm_arch_local_copy(PACMAN_LOCAL_PATH);
            if let Ok(handle) = local_head_handle {
                self.local_head_handle = Some(Box::new(handle));
            }
            if let Some(handle) = self.local_head_handle.as_mut() {
                handle.refresh().is_ok() && self.upstream_handle.refresh().is_ok()
            } else {
                self.upstream_handle.refresh().is_ok()
            }
        }
    }

    fn detect_new_pkgs(&self, pkg: &alpm::Package) {
        // skip pkg if exist in local db copy
        if let Some(handle) = &self.local_head_handle {
            if handle.is_package_available(pkg.name()) {
                return;
            }
        }

        let pkg_meta = ffi::PackageMetadata::try_from(pkg);
        if let Err(_) = pkg_meta {
            // NOTE: dirty, but doesn't need to hook tracing/log to userver logging system
            log_error_msg(format!(
                "Failed to get required pkg fields '{}/{}-{}'",
                pkg.db().unwrap().name(),
                pkg.name(),
                pkg.version()
            ));
            return;
        }

        self.insert_or_update_package(
            pkg.db().unwrap().name(),
            pkg.name(),
            pkg.version(),
            pkg.filename().unwrap(),
            &pkg_meta.unwrap(),
            &pkg.into(),
        );
    }

    fn detect_diff_pkgs(&self, local_pkg: &alpm::Package) {
        let pkgname = local_pkg.name();
        let pkgver = local_pkg.version();

        // packages removed from upstream, but present in local copy
        let arch_pkg = self.upstream_handle.find_package(pkgname);
        if arch_pkg.is_none() {
            self.remove_package(local_pkg.db().unwrap().name(), pkgname);
            return;
        }
        let arch_pkg = arch_pkg.unwrap();

        // skip existing version of package
        if arch_pkg.version() == pkgver {
            return;
        }

        // skip pkg if it's missing some fields
        let pkg_meta = ffi::PackageMetadata::try_from(arch_pkg);
        if let Err(_) = pkg_meta {
            // NOTE: dirty, but doesn't need to hook tracing/log to userver logging system
            log_error_msg(format!(
                "Failed to get required pkg fields '{}/{}-{}'",
                arch_pkg.db().unwrap().name(),
                arch_pkg.name(),
                arch_pkg.version()
            ));
            return;
        }

        // version in upstream higher
        if arch_pkg.version() > pkgver {
            self.insert_or_update_package(
                arch_pkg.db().unwrap().name(),
                arch_pkg.name(),
                arch_pkg.version(),
                arch_pkg.filename().unwrap(),
                &pkg_meta.unwrap(),
                &arch_pkg.into(),
            );
        }
    }

    pub fn run_check(&self) -> Result<()> {
        if let Some(handle) = &self.local_head_handle {
            for pkg in handle.pkgcache() {
                self.detect_diff_pkgs(pkg);
            }
        }

        // detect new packages
        for pkg in self.upstream_handle.pkgcache() {
            self.detect_new_pkgs(pkg);
        }
        Ok(())
    }

    fn remove_package(&self, repo_name: &str, pkgname: &str) {
        if !self.gateway.is_null() {
            self.gateway.remove_package_rust(repo_name, pkgname);
        }
    }

    fn insert_or_update_package(
        &self,
        repo_name: &str,
        pkgname: &str,
        pkg_ver: &str,
        pkg_filename: &str,
        meta: &ffi::PackageMetadata,
        deps: &ffi::PackageDependencies,
    ) {
        if !self.gateway.is_null() {
            self.gateway.insert_or_update_package_rust(
                repo_name,
                pkgname,
                pkg_ver,
                pkg_filename,
                meta,
                deps,
            );
        }
    }
}

fn init_alpm(
    db_path: &str,
    rootdir: &str,
    fetch_dbs: bool,
    pacman_dbs: &[DbConfig],
) -> Result<Alpm> {
    let mut handle = Alpm::new(rootdir, db_path).context("Failed to initialize alpm")?;
    for repo in pacman_dbs {
        let db = handle.register_syncdb_mut(&*repo.name, SigLevel::USE_DEFAULT)?;
        db.set_servers(repo.servers.iter())?;
    }

    if !fetch_dbs {
        return Ok(handle);
    }

    let syncdbs = handle.syncdbs_mut();
    if let Err(db_err) = syncdbs.update(false) {
        anyhow::bail!("Failed to update DB! {}", db_err);
    }

    Ok(handle)
}

fn init_alpm_arch_local_copy(pacman_path: &str) -> Result<AlpmManager> {
    let pacman_dbs = vec![
        DbConfig {
            name: "core".into(),
            servers: vec!["file:///var/lib/api-service/archrepo/".into()],
        },
        DbConfig {
            name: "extra".into(),
            servers: vec!["file:///var/lib/api-service/archrepo/".into()],
        },
    ];
    AlpmManager::new(pacman_path.into(), pacman_path.into(), true, pacman_dbs)
}

fn init_alpm_arch(pacman_path: &str) -> Result<AlpmManager> {
    let pacman_dbs = vec![
        DbConfig {
            name: "core".into(),
            servers: vec![
                "https://geo.mirror.pkgbuild.com/core/os/x86_64/".into(),
                "https://mirror.osbeck.com/archlinux/core/os/x86_64/".into(),
            ],
        },
        DbConfig {
            name: "extra".into(),
            servers: vec![
                "https://geo.mirror.pkgbuild.com/extra/os/x86_64/".into(),
                "https://mirror.osbeck.com/archlinux/extra/os/x86_64/".into(),
            ],
        },
    ];
    AlpmManager::new(pacman_path.into(), pacman_path.into(), true, pacman_dbs)
}

pub fn log_error_msg(msg: String) {
    cxx::let_cxx_string!(logging_msg = msg);
    ffi::log_error_msg(&logging_msg);
}

pub fn log_info_msg(msg: String) {
    cxx::let_cxx_string!(logging_msg = msg);
    ffi::log_info_msg(&logging_msg);
}
