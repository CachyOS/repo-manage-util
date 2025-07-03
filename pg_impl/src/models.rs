use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents metadata for a package, mapped to the `helper_schema.package_metadata` composite type
/// in PostgreSQL.
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "helper_schema.package_metadata")]
pub struct PackageMetadata {
    /// The base name of the package (e.g., "linux" for "linux-5.10.1-1").
    pub pkg_base: Option<String>,
    /// A short description of the package.
    pub pkg_desc: Option<String>,
    /// A list of groups the package belongs to.
    pub pkg_groups: Option<Vec<String>>,
    /// The upstream URL for the package.
    pub pkg_url: Option<String>,
    /// A list of licenses under which the package is distributed.
    pub pkg_license: Option<Vec<String>>,
    /// The architecture the package is built for (e.g., "x86_64", "any").
    pub pkg_arch: Option<String>,
    /// The date and time the package was built.
    pub pkg_builddate: Option<DateTime<Utc>>,
    /// The name and email of the package maintainer.
    pub pkg_packager: Option<String>,
    /// The compressed size of the package in bytes.
    pub pkg_csize: Option<i64>,
    /// The installed size of the package in bytes.
    pub pkg_isize: Option<i64>,
    /// The SHA-256 checksum of the package file.
    pub pkg_sha256sum: Option<String>,
    /// The PGP signature of the package file.
    pub pkg_pgpsig: Option<String>,
}

/// Represents the dependencies of a package, mapped to the `helper_schema.package_dependencies`
/// composite type in PostgreSQL.
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "helper_schema.package_dependencies")]
pub struct PackageDependencies {
    /// A list of packages that this package replaces.
    pub pkg_replaces: Option<Vec<String>>,
    /// A list of packages required by this package to run.
    pub pkg_depends: Option<Vec<String>>,
    /// A list of optional dependencies for this package.
    pub pkg_optdepends: Option<Vec<String>>,
    /// A list of packages required to build this package from source.
    pub pkg_makedepends: Option<Vec<String>>,
    /// A list of packages required to run the test suite for this package.
    pub pkg_checkdepends: Option<Vec<String>>,
    /// A list of packages that conflict with this package.
    pub pkg_conflicts: Option<Vec<String>>,
    /// A list of virtual packages provided by this package.
    pub pkg_provides: Option<Vec<String>>,
    /// A list of important files included in the package.
    pub pkg_files: Option<Vec<String>>,
}

/// Represents information about a repository, mapped to the `helper_schema.repository_info`
/// composite type in PostgreSQL.
///
/// This is used as an argument type for database functions.
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "helper_schema.repository_info")]
pub struct RepositoryInfo {
    /// A short description of the repository.
    pub repo_desc: Option<String>,
}

/// Represents a repository record from the database.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct Repository {
    /// The unique identifier for the repository.
    pub id: Option<Uuid>,
    /// The name of the repository (e.g., "core", "extra").
    pub repo_name: Option<String>,
    /// A short description of the repository.
    pub repo_desc: Option<String>,
    /// The timestamp when the repository was first created.
    pub created_at: Option<DateTime<Utc>>,
    /// The timestamp when the repository was last updated.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Represents a complete package record from the database, including all metadata and dependencies.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct Package {
    /// The unique identifier for the package entry.
    pub id: Option<Uuid>,
    /// The name of the repository this package belongs to.
    pub repo_name: Option<String>,
    /// The name of the package (e.g., "pacman").
    pub pkg_name: Option<String>,
    /// The version of the package (e.g., "6.0.2-7").
    pub pkg_version: Option<String>,
    /// The full filename of the package archive (e.g., "pacman-6.0.2-7-x86_64.pkg.tar.zst").
    pub pkg_filename: Option<String>,
    /// The base name of the package (e.g., "pacman").
    pub pkg_base: Option<String>,
    /// A short description of the package.
    pub pkg_desc: Option<String>,
    /// A list of groups the package belongs to.
    pub pkg_groups: Option<Vec<String>>,
    /// The upstream URL for the package.
    pub pkg_url: Option<String>,
    /// A list of licenses under which the package is distributed.
    pub pkg_license: Option<Vec<String>>,
    /// The architecture the package is built for (e.g., "x86_64", "any").
    pub pkg_arch: Option<String>,
    /// The date and time the package was built.
    pub pkg_builddate: Option<DateTime<Utc>>,
    /// The name and email of the package maintainer.
    pub pkg_packager: Option<String>,
    /// The compressed size of the package in bytes.
    pub pkg_csize: Option<i64>,
    /// The installed size of the package in bytes.
    pub pkg_isize: Option<i64>,
    /// The SHA-256 checksum of the package file.
    pub pkg_sha256sum: Option<String>,
    /// The PGP signature of the package file.
    pub pkg_pgpsig: Option<String>,
    /// A list of packages that this package replaces.
    pub pkg_replaces: Option<Vec<String>>,
    /// A list of packages required by this package to run.
    pub pkg_depends: Option<Vec<String>>,
    /// A list of optional dependencies for this package.
    pub pkg_optdepends: Option<Vec<String>>,
    /// A list of packages required to build this package from source.
    pub pkg_makedepends: Option<Vec<String>>,
    /// A list of packages required to run the test suite for this package.
    pub pkg_checkdepends: Option<Vec<String>>,
    /// A list of packages that conflict with this package.
    pub pkg_conflicts: Option<Vec<String>>,
    /// A list of virtual packages provided by this package.
    pub pkg_provides: Option<Vec<String>>,
    /// A list of important files included in the package.
    pub pkg_files: Option<Vec<String>>,
    /// The timestamp when this package record was last updated in the database.
    pub updated: Option<DateTime<Utc>>,
}

/// Represents a summary of statistics for a repository.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct RepoSummary {
    /// The name of the repository.
    pub repo_name: Option<String>,
    /// The total number of package entries in the repository (including multiple versions).
    pub total_packages: Option<i64>,
    /// The number of unique packages (by name) in the repository.
    pub unique_packages: Option<i64>,
    /// The timestamp of the oldest package in the repository.
    pub oldest_package_update: Option<DateTime<Utc>>,
    /// The timestamp of the newest package in the repository.
    pub newest_package_update: Option<DateTime<Utc>>,
    /// A short description of the repository.
    pub repo_desc: Option<String>,
}
