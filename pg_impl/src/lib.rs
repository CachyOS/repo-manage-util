pub mod db;
pub mod error;
pub mod models;

#[cfg(test)]
mod tests {
    use crate::db::Db;
    use crate::models::{PackageDependencies, PackageMetadata, RepositoryInfo};
    use sqlx::PgPool;

    // Helper function to create a default package for testing
    fn create_test_package_parts(
        name: &str,
        version: &str,
    ) -> (String, String, PackageMetadata, PackageDependencies) {
        let filename = format!("{name}-{version}-x86_64.pkg.tar.zst");
        let metadata = PackageMetadata {
            pkg_base: Some(name.to_string()),
            pkg_desc: Some(format!("Description for {name}")),
            pkg_groups: None,
            pkg_url: Some("http://example.com".to_string()),
            pkg_license: Some(vec!["MIT".to_string()]),
            pkg_arch: Some("x86_64".to_string()),
            pkg_builddate: Some(chrono::Utc::now()),
            pkg_packager: Some("Test Packager".to_string()),
            pkg_csize: Some(1024),
            pkg_isize: Some(4096),
            pkg_sha256sum: Some(format!("sha256-{name}")),
            pkg_pgpsig: None,
        };
        let dependencies = PackageDependencies {
            pkg_replaces: None,
            pkg_depends: Some(vec!["glibc".to_string()]),
            pkg_optdepends: None,
            pkg_makedepends: None,
            pkg_checkdepends: None,
            pkg_conflicts: None,
            pkg_provides: Some(vec![name.to_string()]),
            pkg_files: Some(vec![format!("/usr/bin/{name}")]),
        };
        (filename, version.to_string(), metadata, dependencies)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_repository_lifecycle(pool: PgPool) {
        let db = Db::connect_from_pgpool(pool).await.unwrap();
        let repo_name = "core-testing";

        // 1. Insert
        let repo_info =
            Some(RepositoryInfo { repo_desc: Some("Core testing repository".to_string()) });
        let repo_id = db.insert_or_update_repository(repo_name, repo_info).await.unwrap();
        assert!(!repo_id.is_nil());

        // 2. Retrieve and verify
        let repos = db.get_all_repositories().await.unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repo_name, Some(repo_name.into()));
        assert_eq!(repos[0].repo_desc.as_deref(), Some("Core testing repository"));

        // 3. Update
        let updated_repo_info =
            Some(RepositoryInfo { repo_desc: Some("An updated description".to_string()) });
        let updated_repo_id =
            db.insert_or_update_repository(repo_name, updated_repo_info).await.unwrap();
        assert_eq!(repo_id, updated_repo_id); // Should be the same repo

        // 4. Retrieve and verify update
        let repos = db.get_all_repositories().await.unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repo_desc.as_deref(), Some("An updated description"));

        // Add a second repo to ensure we only delete the correct one
        db.insert_or_update_repository("extra", None).await.unwrap();
        assert_eq!(db.get_all_repositories().await.unwrap().len(), 2);

        // 5. Remove repository twice (case when it exist, and when it doesn't)
        db.remove_existing_repository(repo_name).await.unwrap();
        db.remove_existing_repository(repo_name).await.unwrap();

        // 6. Verify removal
        let repos_after_delete = db.get_all_repositories().await.unwrap();
        assert_eq!(repos_after_delete.len(), 1);
        assert_eq!(repos_after_delete[0].repo_name, Some("extra".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_package_lifecycle(pool: PgPool) {
        let db = Db::connect_from_pgpool(pool).await.unwrap();
        let repo_name = "core";
        db.insert_or_update_repository(repo_name, None).await.unwrap();

        let pkg_name = "test-package";
        let (filename, version, metadata, dependencies) =
            create_test_package_parts(pkg_name, "1.0.0-1");

        // 1. Insert package
        let pkg_id = db
            .insert_or_update_package(
                repo_name,
                pkg_name,
                &version,
                &filename,
                metadata.clone(),
                dependencies,
            )
            .await
            .unwrap();
        assert!(!pkg_id.is_nil());

        // 2. Get package and verify
        let pkg_opt = db.get_package_info(repo_name, pkg_name).await.unwrap();
        assert!(pkg_opt.is_some());
        let pkg = pkg_opt.unwrap();
        assert_eq!(pkg.pkg_name, Some(pkg_name.into()));
        assert_eq!(pkg.pkg_version, Some(version.clone()));
        assert_eq!(pkg.pkg_license, metadata.pkg_license);

        // 3. Update package (new version, should update in place)
        let (new_filename, new_version, mut new_metadata, new_deps) =
            create_test_package_parts(pkg_name, "1.0.0-2");
        new_metadata.pkg_desc = Some("An updated description".to_string());

        let updated_pkg_id = db
            .insert_or_update_package(
                repo_name,
                pkg_name,
                &new_version,
                &new_filename,
                new_metadata,
                new_deps,
            )
            .await
            .unwrap();
        assert_eq!(pkg_id, updated_pkg_id); // ON CONFLICT should return same ID

        let pkg_opt = db.get_package_info(repo_name, pkg_name).await.unwrap();
        assert!(pkg_opt.is_some());
        let pkg = pkg_opt.unwrap();
        assert_eq!(pkg.pkg_desc.as_deref(), Some("An updated description"));
        assert_eq!(pkg.pkg_version.as_deref(), Some("1.0.0-2"));

        // 4. Remove package
        let was_removed = db.remove_package(repo_name, pkg_name).await.unwrap();
        assert!(was_removed);

        // 5. Verify removal
        let pkg_opt = db.get_package_info(repo_name, pkg_name).await.unwrap();
        assert!(pkg_opt.is_none());

        let was_removed_again = db.remove_package(repo_name, pkg_name).await.unwrap();
        assert!(!was_removed_again);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_package_search_and_filter(pool: PgPool) {
        let db = Db::connect_from_pgpool(pool).await.unwrap();
        let repo_name = "extra";
        db.insert_or_update_repository(repo_name, None).await.unwrap();

        // Insert some packages
        let (fname1, ver1, meta1, deps1) = create_test_package_parts("awesome-tool", "1.0");
        db.insert_or_update_package(repo_name, "awesome-tool", &ver1, &fname1, meta1, deps1)
            .await
            .unwrap();

        let (fname2, ver2, mut meta2, deps2) = create_test_package_parts("another-app", "2.0");
        meta2.pkg_desc = Some("A very awesome application.".to_string());
        db.insert_or_update_package(repo_name, "another-app", &ver2, &fname2, meta2, deps2)
            .await
            .unwrap();

        let (fname3, ver3, mut meta3, deps3) = create_test_package_parts("system-lib", "3.0");
        meta3.pkg_arch = Some("aarch64".to_string());
        db.insert_or_update_package(repo_name, "system-lib", &ver3, &fname3, meta3, deps3)
            .await
            .unwrap();

        // Test get_repo_packages
        let all_pkgs = db.get_repo_packages(repo_name).await.unwrap();
        assert_eq!(all_pkgs.len(), 3);

        // Test search_packages (should match name and description)
        let search_results = db.search_packages(repo_name, "awesome").await.unwrap();
        assert_eq!(search_results.len(), 2);
        assert!(search_results.iter().any(|p| p.pkg_name == Some("awesome-tool".into())));
        assert!(search_results.iter().any(|p| p.pkg_name == Some("another-app".into())));

        // Test get_packages_by_arch
        let x86_pkgs = db.get_packages_by_arch(repo_name, "x86_64").await.unwrap();
        assert_eq!(x86_pkgs.len(), 2);
        assert!(x86_pkgs.iter().all(|p| p.pkg_arch == Some("x86_64".to_string())));

        let aarch64_pkgs = db.get_packages_by_arch(repo_name, "aarch64").await.unwrap();
        assert_eq!(aarch64_pkgs.len(), 1);
        assert_eq!(aarch64_pkgs[0].pkg_name, Some("system-lib".to_string()));

        let arm_pkgs = db.get_packages_by_arch(repo_name, "armv7h").await.unwrap();
        assert!(arm_pkgs.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_repo_stats(pool: PgPool) {
        let db = Db::connect_from_pgpool(pool).await.unwrap();
        let repo_name = "testing-stats";
        db.insert_or_update_repository(repo_name, None).await.unwrap();

        // No packages yet.
        let stats_empty = db.get_repo_stats(repo_name).await.unwrap();
        assert!(stats_empty.is_empty());

        // Add two packages, one with two versions
        let (fname1, ver1, mut meta1, deps1) = create_test_package_parts("stats-pkg-1", "1.0");
        meta1.pkg_csize = Some(1000);
        db.insert_or_update_package(repo_name, "stats-pkg-1", &ver1, &fname1, meta1, deps1)
            .await
            .unwrap();

        let (fname2, ver2, mut meta2, deps2) = create_test_package_parts("stats-pkg-2", "2.0");
        meta2.pkg_csize = Some(2500);
        db.insert_or_update_package(repo_name, "stats-pkg-2", &ver2, &fname2, meta2, deps2)
            .await
            .unwrap();

        let (fname3, ver3, mut meta3, deps3) = create_test_package_parts("stats-pkg-2", "2.1");
        meta3.pkg_csize = Some(3000);
        db.insert_or_update_package(repo_name, "stats-pkg-2", &ver3, &fname3, meta3, deps3)
            .await
            .unwrap();

        // Get stats
        let stats = db.get_repo_stats(repo_name).await.unwrap();
        assert_eq!(stats.len(), 1);
        let summary = &stats[0];
        assert_eq!(summary.repo_name, Some(repo_name.into()));
        assert_eq!(summary.total_packages, Some(2));
        assert_eq!(summary.unique_packages, Some(2));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_reindex_packages_table_empty(pool: PgPool) {
        let db = Db::connect_from_pgpool(pool).await.unwrap();

        // Reindex on an empty table should succeed without error
        db.reindex_packages_table().await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_reindex_packages_table_with_data(pool: PgPool) {
        let db = Db::connect_from_pgpool(pool).await.unwrap();
        let repo_name = "core";
        db.insert_or_update_repository(repo_name, None).await.unwrap();

        // Insert several packages to populate indexes
        for i in 0..5 {
            let (filename, version, metadata, dependencies) =
                create_test_package_parts(&format!("reindex-pkg-{i}"), "1.0.0-1");
            db.insert_or_update_package(
                repo_name,
                &format!("reindex-pkg-{i}"),
                &version,
                &filename,
                metadata,
                dependencies,
            )
            .await
            .unwrap();
        }

        // Reindex should succeed with data present
        db.reindex_packages_table().await.unwrap();

        // Verify all data is still intact and queryable after reindex
        let all_pkgs = db.get_repo_packages(repo_name).await.unwrap();
        assert_eq!(all_pkgs.len(), 5);

        // Verify index-dependent operations still work (search uses GIN index)
        let search_results = db.search_packages(repo_name, "reindex-pkg-0").await.unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].pkg_name, Some("reindex-pkg-0".to_string()));

        // Verify inserts still work after reindex (uses idx_packages_name)
        let (filename, version, metadata, dependencies) =
            create_test_package_parts("post-reindex-pkg", "1.0.0-1");
        let id = db
            .insert_or_update_package(
                repo_name,
                "post-reindex-pkg",
                &version,
                &filename,
                metadata,
                dependencies,
            )
            .await
            .unwrap();
        assert!(!id.is_nil());

        let all_pkgs = db.get_repo_packages(repo_name).await.unwrap();
        assert_eq!(all_pkgs.len(), 6);
    }
}
