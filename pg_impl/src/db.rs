use crate::error::Result;
use crate::models::{
    Package, PackageDependencies, PackageMetadata, RepoSummary, Repository, RepositoryInfo,
};

use sqlx::postgres::PgConnectOptions;
use sqlx::{ConnectOptions, PgPool};
use uuid::Uuid;

/// A database access layer for managing repositories and packages.
///
/// This struct encapsulates a `PgPool` and provides high-level methods
/// for interacting with the database, abstracting away the underlying SQL queries.
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Creates a new database connection pool.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use std::env;
    /// # use pg_impl::{db::Db, error::Result};
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    ///     let db = Db::connect(&database_url).await?;
    ///     println!("Successfully connected to the database.");
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(database_url: &str) -> Result<Self> {
        let connection_options =
            database_url.parse::<PgConnectOptions>()?.disable_statement_logging();
        let pool = PgPool::connect_with(connection_options).await?;
        Ok(Db { pool })
    }

    /// Creates a new `Db` instance from an existing `PgPool`.
    ///
    /// This is useful when you want to share a connection pool across different parts of your application.
    pub async fn connect_from_pgpool(pool: PgPool) -> Result<Self> {
        Ok(Db { pool })
    }

    /// Migrates the database to the latest version.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use std::env;
    /// # use pg_impl::{db::Db, error::Result};
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    ///     let db = Db::connect(&database_url).await?;
    ///     db.migrate().await?;
    ///     println!("Database migration completed successfully.");
    ///     Ok(())
    /// }
    /// ```
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Inserts a new repository or updates an existing one based on its name.
    ///
    /// If a repository with the given name exists, its description is updated.
    /// Otherwise, a new repository is created.
    ///
    /// Returns the id of the created or updated repository.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result, models::RepositoryInfo};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let repo_info = Some(RepositoryInfo {
    ///     repo_desc: Some("The main official repository.".to_string()),
    /// });
    /// let repo_id = db.insert_or_update_repository("core", repo_info).await?;
    /// println!("Repository 'core' has ID: {}", repo_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert_or_update_repository(
        &self,
        name: &str,
        info: Option<RepositoryInfo>,
    ) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            "SELECT helper_schema.insert_or_update_repository($1, $2)",
            name,
            info as Option<RepositoryInfo>
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(id.unwrap())
    }

    /// Removes a repository and all of its associated packages.
    ///
    /// This is a destructive operation and cannot be undone.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// db.remove_existing_repository("staging").await?;
    /// println!("Repository 'staging' and all its packages have been removed.");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_existing_repository(&self, repo_name: &str) -> Result<()> {
        let _ = sqlx::query!("SELECT helper_schema.remove_repository($1)", repo_name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(())
    }

    /// Retrieves all repositories from the database.
    ///
    /// Returns `Vec<Repository>` containing all repositories.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let repositories = db.get_all_repositories().await?;
    /// for repo in repositories {
    ///     println!("- {}: {}", repo.repo_name.unwrap_or_default(), repo.repo_desc.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_all_repositories(&self) -> Result<Vec<Repository>> {
        let repos =
            sqlx::query_as!(Repository, "SELECT * FROM helper_schema.get_all_repositories()")
                .fetch_all(&self.pool)
                .await?;
        Ok(repos)
    }

    /// Inserts a new package or updates an existing one.
    ///
    /// A package is uniquely identified by its name, version, and repository.
    /// If a package with the same details exists, its metadata/depends updated.
    ///
    /// Returns the id of the created or updated package.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result, models::{PackageMetadata, PackageDependencies}};
    /// # use chrono::Utc;
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let metadata = PackageMetadata {
    ///     pkg_base: Some("pacman".to_string()),
    ///     pkg_desc: Some("A package manager for Arch Linux".to_string()),
    ///     pkg_groups: None,
    ///     pkg_url: Some("https://archlinux.org/pacman/".to_string()),
    ///     pkg_license: Some(vec!["GPL".to_string()]),
    ///     pkg_arch: Some("x86_64".to_string()),
    ///     pkg_builddate: Some(Utc::now()),
    ///     pkg_packager: Some("John Doe <john.doe@example.com>".to_string()),
    ///     pkg_csize: Some(102400),
    ///     pkg_isize: Some(512000),
    ///     pkg_sha256sum: Some("...".to_string()),
    ///     pkg_pgpsig: Some("...".to_string()),
    /// };
    /// let dependencies = PackageDependencies {
    ///     pkg_replaces: None,
    ///     pkg_depends: Some(vec!["glibc".to_string(), "bash".to_string()]),
    ///     pkg_optdepends: None,
    ///     pkg_makedepends: None,
    ///     pkg_checkdepends: None,
    ///     pkg_conflicts: None,
    ///     pkg_provides: None,
    ///     pkg_files: None,
    /// };
    ///
    /// let pkg_id = db.insert_or_update_package(
    ///     "core",
    ///     "pacman",
    ///     "6.0.2-7",
    ///     "pacman-6.0.2-7-x86_64.pkg.tar.zst",
    ///     metadata,
    ///     dependencies
    /// ).await?;
    /// println!("Package 'pacman' has ID: {pkg_id}");
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_or_update_package(
        &self,
        repo_name: &str,
        pkg_name: &str,
        pkg_version: &str,
        pkg_filename: &str,
        metadata: PackageMetadata,
        dependencies: PackageDependencies,
    ) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            "SELECT helper_schema.insert_or_update_package($1, $2, $3, $4, $5, $6)",
            repo_name,
            pkg_name,
            pkg_version,
            pkg_filename,
            metadata as PackageMetadata,
            dependencies as PackageDependencies
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(id.unwrap())
    }

    /// Removes a specific version of a package from a repository.
    ///
    /// Returns `true` if a package was found and deleted, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let was_removed = db.remove_package("core-testing", "linux", "5.15.1-1").await?;
    /// if was_removed {
    ///     println!("Successfully removed linux-5.15.1-1 from core-testing.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_package(
        &self,
        repo_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> Result<bool> {
        let was_removed = sqlx::query_scalar!(
            "SELECT helper_schema.remove_package($1, $2, $3)",
            repo_name,
            pkg_name,
            pkg_version
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(was_removed.unwrap_or(false))
    }

    /// Removes a stale package from a repository.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// db.remove_stale_package("extra", "awesome-wm").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_stale_package(&self, repo_name: &str, pkg_name: &str) -> Result<()> {
        let _ =
            sqlx::query!("SELECT helper_schema.remove_stale_package($1, $2)", repo_name, pkg_name,)
                .fetch_optional(&self.pool)
                .await?;
        Ok(())
    }

    /// Gets all available versions of a specific package from a repository.
    ///
    /// Returns `Vec<Package>` containing all versions of the specified package.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let versions = db.get_package_info("core", "glibc").await?;
    /// println!("Available versions of glibc in core:");
    /// for pkg in versions {
    ///     println!("- {}", pkg.pkg_version.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_package_info(&self, repo_name: &str, pkg_name: &str) -> Result<Vec<Package>> {
        let packages = sqlx::query_as!(
            Package,
            "SELECT * FROM helper_schema.get_package_info($1, $2)",
            repo_name,
            pkg_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(packages)
    }

    /// Gets all packages of a specific architecture from a repository.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let any_arch_packages = db.get_packages_by_arch("extra", "any").await?;
    /// println!("'any' architecture packages in extra: {}", any_arch_packages.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_packages_by_arch(
        &self,
        repo_name: &str,
        pkg_arch: &str,
    ) -> Result<Vec<Package>> {
        let packages = sqlx::query_as!(
            Package,
            "SELECT * FROM helper_schema.get_packages_by_arch($1, $2)",
            repo_name,
            pkg_arch
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(packages)
    }

    /// Gets all packages within a specific repository.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let all_core_packages = db.get_repo_packages("core").await?;
    /// println!("Total packages in 'core' repository: {}", all_core_packages.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_repo_packages(&self, repo_name: &str) -> Result<Vec<Package>> {
        let packages = sqlx::query_as!(
            Package,
            "SELECT * FROM helper_schema.get_repo_packages($1)",
            repo_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(packages)
    }

    /// Searches for packages in a repository by name or description.
    ///
    /// The search is typically case-insensitive and matches partial strings.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let results = db.search_packages("extra", "web server").await?;
    /// println!("Found packages matching 'web server':");
    /// for pkg in results {
    ///     println!("- {} ({}): {}",
    ///         pkg.pkg_name.unwrap_or_default(),
    ///         pkg.pkg_version.unwrap_or_default(),
    ///         pkg.pkg_desc.unwrap_or_default()
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_packages(
        &self,
        repo_name: &str,
        search_pattern: &str,
    ) -> Result<Vec<Package>> {
        let packages = sqlx::query_as!(
            Package,
            "SELECT * FROM helper_schema.search_packages($1, $2)",
            repo_name,
            search_pattern
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(packages)
    }

    /// Retrieves the latest version of each unique package in a repository.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// let latest_packages = db.get_latest_packages("extra").await?;
    /// println!("Latest packages in 'extra': {}", latest_packages.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_latest_packages(&self, repo_name: &str) -> Result<Vec<Package>> {
        let packages = sqlx::query_as!(
            Package,
            "SELECT * FROM helper_schema.get_latest_packages($1)",
            repo_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(packages)
    }

    /// Deletes old package versions, keeping a specified number of recent versions for each package.
    ///
    /// Returns the total number of deleted package records.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// // Keep only the 3 most recent versions of each package in 'core'
    /// let deleted_count = db.cleanup_old_package_versions("core", 3).await?;
    /// println!("Cleaned up {} old package versions from 'core'.", deleted_count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cleanup_old_package_versions(
        &self,
        repo_name: &str,
        versions_to_keep: i32,
    ) -> Result<i32> {
        let deleted_count = sqlx::query_scalar!(
            "SELECT helper_schema.cleanup_old_package_versions($1, $2)",
            repo_name,
            versions_to_keep
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(deleted_count.unwrap_or(0))
    }

    /// Retrieves statistics for a specific repository.
    ///
    /// Returns `Vec<RepoSummary>` containing the statistics.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pg_impl::{db::Db, error::Result};
    /// # async fn run() -> Result<()> {
    /// # let db = Db::connect("...").await?;
    /// if let Some(summary) = db.get_repo_stats("core").await?.first() {
    ///     println!("Stats for 'core':");
    ///     println!("  Total packages: {}", summary.total_packages.unwrap_or(0));
    ///     println!("  Unique packages: {}", summary.unique_packages.unwrap_or(0));
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_repo_stats(&self, repo_name: &str) -> Result<Vec<RepoSummary>> {
        let stats = sqlx::query_as!(
            RepoSummary,
            "SELECT * FROM helper_schema.get_repo_stats($1)",
            repo_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(stats)
    }
}
