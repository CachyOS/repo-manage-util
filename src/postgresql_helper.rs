use crate::pg_types::{
    PackageDependencies, PackageInfo, PackageMetadata, RepoSummary, Repository, RepositoryInfo,
};

use anyhow::{Context, Result};
use pg_impl::db;
use uuid::Uuid;

pub struct PostgresqlHelper {
    db: db::Db,
}

impl PostgresqlHelper {
    /// Create a new `PostgreSQL` helper with connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = db::Db::connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL database")?;

        // migrate database
        db.migrate().await.context("Failed to migrate database")?;

        Ok(Self { db })
    }

    /// Insert or update a package in the database.
    ///
    /// If the operation fails due to index corruption (e.g. "cannot find insert offset"),
    /// automatically reindexes the packages table and retries once.
    pub async fn insert_or_update_package(
        &self,
        repo_name: &str,
        pkg_name: &str,
        pkg_version: &str,
        pkg_filename: &str,
        metadata: PackageMetadata,
        dependencies: PackageDependencies,
    ) -> Result<Uuid> {
        let meta_pg: pg_impl::models::PackageMetadata = metadata.into();
        let deps_pg: pg_impl::models::PackageDependencies = dependencies.into();

        let result = self
            .db
            .insert_or_update_package(
                repo_name,
                pkg_name,
                pkg_version,
                pkg_filename,
                meta_pg.clone(),
                deps_pg.clone(),
            )
            .await;

        let package_id = match result {
            Ok(id) => id,
            Err(ref err) if err.to_string().contains("cannot find insert offset") => {
                tracing::warn!(
                    "Detected corrupted index on packages table, reindexing and retrying..."
                );
                self.db
                    .reindex_packages_table()
                    .await
                    .context("Failed to reindex packages table")?;
                tracing::info!("Reindex of packages table completed successfully");

                self.db
                    .insert_or_update_package(
                        repo_name,
                        pkg_name,
                        pkg_version,
                        pkg_filename,
                        meta_pg,
                        deps_pg,
                    )
                    .await
                    .context(anyhow::anyhow!(
                        "Failed to insert or update package after reindex: {pkg_name}"
                    ))?
            },
            Err(err) => {
                return Err(err)
                    .context(anyhow::anyhow!("Failed to insert or update package: {pkg_name}"));
            },
        };

        tracing::debug!("'{repo_name}/{pkg_name}-{pkg_version}' ins/upd");
        Ok(package_id)
    }

    /// Remove packages from the database.
    /// Returns true if a package was deleted.
    pub async fn remove_package(&self, repo_name: &str, pkg_name: &str) -> Result<bool> {
        let removed = self
            .db
            .remove_package(repo_name, pkg_name)
            .await
            .context("Failed to remove package")?;

        if removed {
            tracing::debug!("'{repo_name}/{pkg_name}' removed from database");
        } else {
            tracing::debug!("'{repo_name}/{pkg_name}' not found in database");
        }

        Ok(removed)
    }

    /// Removes a packages from a repository.
    pub async fn remove_packages(&self, repo_name: &str, stale_pkgs: &[String]) -> Result<()> {
        for pkg_name in stale_pkgs {
            if let Err(err) = self.remove_package(repo_name, pkg_name).await {
                tracing::error!("Failed to remove '{pkg_name}' from '{repo_name}: {err}");
            }
        }

        Ok(())
    }

    /// Gets information about a specific package from a repository.
    pub async fn get_package_info(
        &self,
        repo_name: &str,
        pkg_name: &str,
    ) -> Result<Option<PackageInfo>> {
        let row = self
            .db
            .get_package_info(repo_name, pkg_name)
            .await
            .context("Failed to get package info")?;

        let pkg_info = row.map(std::convert::Into::into);
        Ok(pkg_info)
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

    /// Inserts or updates a repository.
    pub async fn insert_or_update_repository(
        &self,
        repo_name: &str,
        info: Option<RepositoryInfo>,
    ) -> Result<Uuid> {
        let repo_id = self
            .db
            .insert_or_update_repository(repo_name, info.map(std::convert::Into::into))
            .await
            .context("Failed to insert or update repository")?;

        tracing::debug!("Repository '{repo_name}' ins/upd");
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
}
