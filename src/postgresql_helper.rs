use crate::pg_types::*;

use anyhow::{Context, Result};
use pg_impl::db;
use uuid::Uuid;

pub struct PostgresqlHelper {
    db: db::Db,
}

impl PostgresqlHelper {
    /// Create a new PostgreSQL helper with connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = db::Db::connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL database")?;

        // migrate database
        db.migrate().await.context("Failed to migrate database")?;

        Ok(Self { db })
    }

    /// Insert or update a package in the database
    pub async fn insert_or_update_package(
        &self,
        repo_name: &str,
        pkg_name: &str,
        pkg_version: &str,
        pkg_filename: &str,
        metadata: PackageMetadata,
        dependencies: PackageDependencies,
    ) -> Result<Uuid> {
        let package_id = self
            .db
            .insert_or_update_package(
                repo_name,
                pkg_name,
                pkg_version,
                pkg_filename,
                metadata.into(),
                dependencies.into(),
            )
            .await
            .context("Failed to insert or update package")?;

        log::debug!("'{repo_name}/{pkg_name}-{pkg_version}' ins/upd");
        Ok(package_id)
    }

    /// Removes a specific version of a package from a repository.
    /// Returns true if a package was deleted.
    pub async fn remove_package(
        &self,
        repo_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> Result<bool> {
        let removed = self
            .db
            .remove_package(repo_name, pkg_name, pkg_version)
            .await
            .context("Failed to remove package")?;

        if removed {
            log::debug!("'{repo_name}/{pkg_name}-{pkg_version}' removed from database");
        } else {
            log::debug!("'{repo_name}/{pkg_name}-{pkg_version}' not found in database");
        }

        Ok(removed)
    }

    /// Gets all versions of a specific package from a repository.
    pub async fn get_package_info(
        &self,
        repo_name: &str,
        pkg_name: &str,
    ) -> Result<Vec<PackageInfo>> {
        let rows = self
            .db
            .get_package_info(repo_name, pkg_name)
            .await
            .context("Failed to get package info")?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(row.into());
        }

        Ok(packages)
    }

    /// Gets all packages within a specific repository.
    pub async fn get_repo_packages(&self, repo_name: &str) -> Result<Vec<PackageInfo>> {
        let rows = self
            .db
            .get_repo_packages(repo_name)
            .await
            .context("Failed to get repository packages")?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(row.into());
        }

        Ok(packages)
    }

    /// Retrieves statistics for a specific repository.
    pub async fn get_repo_stats(&self, repo_name: &str) -> Result<Vec<RepoSummary>> {
        let rows = self
            .db
            .get_repo_stats(repo_name)
            .await
            .context("Failed to get repository statistics")?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(row.into());
        }

        Ok(stats)
    }

    /// Searches for packages in a repository by name or description.
    pub async fn search_packages(
        &self,
        repo_name: &str,
        search_pattern: &str,
    ) -> Result<Vec<PackageInfo>> {
        let rows = self
            .db
            .search_packages(repo_name, search_pattern)
            .await
            .context("Failed to search packages")?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(row.into());
        }

        Ok(packages)
    }

    /// Get packages by architecture /// Get packages by architecture
    pub async fn get_packages_by_arch(
        &self,
        repo_name: &str,
        arch: &str,
    ) -> Result<Vec<PackageInfo>> {
        let rows = self
            .db
            .get_packages_by_arch(repo_name, arch)
            .await
            .context("Failed to get packages by architecture")?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(row.into());
        }

        Ok(packages)
    }

    /// Retrieves the latest version of each package in a repository.
    pub async fn get_latest_packages(&self, repo_name: &str) -> Result<Vec<PackageInfo>> {
        let rows = self
            .db
            .get_latest_packages(repo_name)
            .await
            .context("Failed to get latest packages")?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(row.into());
        }

        Ok(packages)
    }

    /// Deletes old package versions, keeping a specified number of recent versions.
    /// Returns the number of deleted package records.
    pub async fn cleanup_old_package_versions(
        &self,
        repo_name: &str,
        keep_versions: i32,
    ) -> Result<i32> {
        let deleted_count = self
            .db
            .cleanup_old_package_versions(repo_name, keep_versions)
            .await
            .context("Failed to cleanup old package versions")?;

        log::info!("Cleaned up {deleted_count} old package versions from repository {repo_name}",);
        Ok(deleted_count)
    }

    /// Inserts or updates a repository.
    pub async fn insert_or_update_repository(
        &self,
        repo_name: &str,
        info: Option<RepositoryInfo>,
    ) -> Result<Uuid> {
        let repo_id = self
            .db
            .insert_or_update_repository(repo_name, info.map(|x| x.into()))
            .await
            .context("Failed to insert or update repository")?;

        log::debug!("Repository '{repo_name}' ins/upd");
        Ok(repo_id)
    }

    /// Removes a repository if it exist.
    pub async fn remove_existing_repository(&self, repo_name: &str) -> Result<()> {
        self.db
            .remove_existing_repository(repo_name)
            .await
            .context("Failed to remove existing repository")
    }

    /// Retrieves all repositories from the database.
    pub async fn get_all_repositories(&self) -> Result<Vec<Repository>> {
        let rows =
            self.db.get_all_repositories().await.context("Failed to get all repositories")?;

        let mut repositories = Vec::new();
        for row in rows {
            repositories.push(row.into());
        }

        Ok(repositories)
    }

    /// Remove stale packages from the database.
    pub async fn remove_stale_packages(
        &self,
        repo_name: &str,
        stale_pkgs: &[String],
    ) -> Result<()> {
        for pkg_name in stale_pkgs {
            if let Err(err) = self.db.remove_stale_package(repo_name, pkg_name).await {
                log::error!("Failed to remove '{pkg_name}' from '{repo_name}: {err}");
            }
        }

        Ok(())
    }
}
