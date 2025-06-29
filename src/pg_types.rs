use chrono::{DateTime, Utc};
use pg_impl::models;

#[derive(Debug, Clone, PartialEq)]
pub struct PackageMetadata {
    pub pkg_base: Option<String>,
    pub pkg_desc: Option<String>,
    pub pkg_groups: Option<Vec<String>>,
    pub pkg_url: Option<String>,
    pub pkg_license: Option<Vec<String>>,
    pub pkg_arch: Option<String>,
    pub pkg_builddate: Option<DateTime<Utc>>,
    pub pkg_packager: Option<String>,
    pub pkg_csize: Option<i64>,
    pub pkg_isize: Option<i64>,
    pub pkg_sha256sum: Option<String>,
    pub pkg_pgpsig: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageDependencies {
    pub pkg_replaces: Option<Vec<String>>,
    pub pkg_depends: Option<Vec<String>>,
    pub pkg_optdepends: Option<Vec<String>>,
    pub pkg_makedepends: Option<Vec<String>>,
    pub pkg_checkdepends: Option<Vec<String>>,
    pub pkg_conflicts: Option<Vec<String>>,
    pub pkg_provides: Option<Vec<String>>,
    pub pkg_files: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryInfo {
    pub repo_desc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Repository {
    pub repo_name: Option<String>,
    pub repo_desc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageInfo {
    pub repo_name: String,
    pub pkg_name: String,
    pub pkg_version: String,
    pub pkg_filename: String,
    pub pkg_base: String,
    pub pkg_desc: Option<String>,
    pub pkg_groups: Option<Vec<String>>,
    pub pkg_url: Option<String>,
    pub pkg_license: Option<Vec<String>>,
    pub pkg_arch: Option<String>,
    pub pkg_builddate: Option<DateTime<Utc>>,
    pub pkg_packager: Option<String>,
    pub pkg_csize: Option<i64>,
    pub pkg_isize: Option<i64>,
    pub pkg_sha256sum: Option<String>,
    pub pkg_pgpsig: Option<String>,
    pub pkg_replaces: Option<Vec<String>>,
    pub pkg_depends: Option<Vec<String>>,
    pub pkg_optdepends: Option<Vec<String>>,
    pub pkg_makedepends: Option<Vec<String>>,
    pub pkg_checkdepends: Option<Vec<String>>,
    pub pkg_conflicts: Option<Vec<String>>,
    pub pkg_provides: Option<Vec<String>>,
    pub pkg_files: Option<Vec<String>>,
    pub updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepoSummary {
    pub repo_name: String,
    pub total_packages: Option<i64>,
    pub unique_packages: Option<i64>,
    pub oldest_package_update: Option<DateTime<Utc>>,
    pub newest_package_update: Option<DateTime<Utc>>,
    pub repo_desc: Option<String>,
}

impl From<&alpm::Package> for PackageMetadata {
    fn from(pkg: &alpm::Package) -> Self {
        Self {
            pkg_base: pkg.base().map(|s| s.to_string()),
            pkg_desc: pkg.desc().map(|s| s.to_string()),
            pkg_groups: Some(pkg.groups().iter().map(|s| s.to_string()).collect()),
            pkg_url: pkg.url().map(|s| s.to_string()),
            pkg_license: Some(pkg.licenses().iter().map(|s| s.to_string()).collect()),
            pkg_arch: pkg.arch().map(|s| s.to_string()),
            pkg_builddate: Some(
                DateTime::from_timestamp(pkg.build_date(), 0).expect("invalid timestamp"),
            ),
            pkg_packager: pkg.packager().map(|s| s.to_string()),
            pkg_csize: Some(pkg.size()),
            pkg_isize: Some(pkg.isize()),
            pkg_sha256sum: pkg.sha256sum().map(|s| s.to_string()),
            pkg_pgpsig: pkg.sig().map(|s| String::from_utf8_lossy(s.sig()).to_string()).ok(),
        }
    }
}

impl From<&alpm::Package> for PackageDependencies {
    fn from(pkg: &alpm::Package) -> Self {
        Self {
            pkg_replaces: Some(pkg.replaces().into_iter().map(|s| s.to_string()).collect()),
            pkg_depends: Some(pkg.depends().into_iter().map(|s| s.to_string()).collect()),
            pkg_optdepends: Some(pkg.optdepends().into_iter().map(|s| s.to_string()).collect()),
            pkg_makedepends: Some(pkg.makedepends().into_iter().map(|s| s.to_string()).collect()),
            pkg_checkdepends: Some(pkg.checkdepends().into_iter().map(|s| s.to_string()).collect()),
            pkg_conflicts: Some(pkg.conflicts().into_iter().map(|s| s.to_string()).collect()),
            pkg_provides: Some(pkg.provides().into_iter().map(|s| s.to_string()).collect()),
            pkg_files: Some(
                pkg.files().files().into_iter().map(|s| s.name().to_string()).collect(),
            ),
        }
    }
}

impl From<models::PackageMetadata> for PackageMetadata {
    fn from(pkg: models::PackageMetadata) -> Self {
        Self {
            pkg_base: pkg.pkg_base,
            pkg_desc: pkg.pkg_desc,
            pkg_groups: pkg.pkg_groups,
            pkg_url: pkg.pkg_url,
            pkg_license: pkg.pkg_license,
            pkg_arch: pkg.pkg_arch,
            pkg_builddate: pkg.pkg_builddate,
            pkg_packager: pkg.pkg_packager,
            pkg_csize: pkg.pkg_csize,
            pkg_isize: pkg.pkg_isize,
            pkg_sha256sum: pkg.pkg_sha256sum,
            pkg_pgpsig: pkg.pkg_pgpsig,
        }
    }
}

impl From<models::PackageDependencies> for PackageDependencies {
    fn from(pkg: models::PackageDependencies) -> Self {
        Self {
            pkg_replaces: pkg.pkg_replaces,
            pkg_depends: pkg.pkg_depends,
            pkg_optdepends: pkg.pkg_optdepends,
            pkg_makedepends: pkg.pkg_makedepends,
            pkg_checkdepends: pkg.pkg_checkdepends,
            pkg_conflicts: pkg.pkg_conflicts,
            pkg_provides: pkg.pkg_provides,
            pkg_files: pkg.pkg_files,
        }
    }
}

impl From<models::RepositoryInfo> for RepositoryInfo {
    fn from(info: models::RepositoryInfo) -> Self {
        Self { repo_desc: info.repo_desc }
    }
}

impl From<models::Repository> for Repository {
    fn from(repo: models::Repository) -> Self {
        Self { repo_name: repo.repo_name, repo_desc: repo.repo_desc }
    }
}

impl From<models::Package> for PackageInfo {
    fn from(pkg: models::Package) -> Self {
        Self {
            repo_name: pkg.repo_name.expect("repo name is required"),
            pkg_name: pkg.pkg_name.expect("Invalid package doesn't have pkgname"),
            pkg_version: pkg.pkg_version.expect("Invalid package doesn't have version"),
            pkg_filename: pkg.pkg_filename.expect("Invalid package doesn't have filename"),
            pkg_base: pkg.pkg_base.expect("Invalid package doesn't have pkgbase"),
            pkg_desc: pkg.pkg_desc,
            pkg_groups: pkg.pkg_groups,
            pkg_url: pkg.pkg_url,
            pkg_license: pkg.pkg_license,
            pkg_arch: pkg.pkg_arch,
            pkg_builddate: pkg.pkg_builddate,
            pkg_packager: pkg.pkg_packager,
            pkg_csize: pkg.pkg_csize,
            pkg_isize: pkg.pkg_isize,
            pkg_sha256sum: pkg.pkg_sha256sum,
            pkg_pgpsig: pkg.pkg_pgpsig,
            pkg_replaces: pkg.pkg_replaces,
            pkg_depends: pkg.pkg_depends,
            pkg_optdepends: pkg.pkg_optdepends,
            pkg_makedepends: pkg.pkg_makedepends,
            pkg_checkdepends: pkg.pkg_checkdepends,
            pkg_conflicts: pkg.pkg_conflicts,
            pkg_provides: pkg.pkg_provides,
            pkg_files: pkg.pkg_files,
            updated: pkg.updated,
        }
    }
}

impl From<models::RepoSummary> for RepoSummary {
    fn from(repo: models::RepoSummary) -> Self {
        Self {
            repo_name: repo.repo_name.expect("repo name is required"),
            total_packages: repo.total_packages,
            unique_packages: repo.unique_packages,
            oldest_package_update: repo.oldest_package_update,
            newest_package_update: repo.newest_package_update,
            repo_desc: repo.repo_desc,
        }
    }
}

impl From<PackageDependencies> for models::PackageDependencies {
    fn from(pkg: PackageDependencies) -> Self {
        Self {
            pkg_replaces: pkg.pkg_replaces,
            pkg_depends: pkg.pkg_depends,
            pkg_optdepends: pkg.pkg_optdepends,
            pkg_makedepends: pkg.pkg_makedepends,
            pkg_checkdepends: pkg.pkg_checkdepends,
            pkg_conflicts: pkg.pkg_conflicts,
            pkg_provides: pkg.pkg_provides,
            pkg_files: pkg.pkg_files,
        }
    }
}

impl From<PackageMetadata> for models::PackageMetadata {
    fn from(pkg: PackageMetadata) -> Self {
        Self {
            pkg_base: pkg.pkg_base,
            pkg_desc: pkg.pkg_desc,
            pkg_groups: pkg.pkg_groups,
            pkg_url: pkg.pkg_url,
            pkg_license: pkg.pkg_license,
            pkg_arch: pkg.pkg_arch,
            pkg_builddate: pkg.pkg_builddate,
            pkg_packager: pkg.pkg_packager,
            pkg_csize: pkg.pkg_csize,
            pkg_isize: pkg.pkg_isize,
            pkg_sha256sum: pkg.pkg_sha256sum,
            pkg_pgpsig: pkg.pkg_pgpsig,
        }
    }
}

impl From<RepositoryInfo> for models::RepositoryInfo {
    fn from(info: RepositoryInfo) -> Self {
        Self { repo_desc: info.repo_desc }
    }
}
