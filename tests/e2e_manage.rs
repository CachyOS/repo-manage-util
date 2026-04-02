mod common;

use std::fs;

use tempfile::TempDir;

use common::*;

#[test]
fn reset_creates_db_from_packages() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "bar", "2.0-1", "x86_64");

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    assert!(db_path.exists(), "DB file should be created");
    assert_db_contains(&db_path, "foo");
    assert_db_contains(&db_path, "bar");
}

#[test]
fn reset_empty_repo() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();
}

#[test]
fn reset_keeps_only_latest_versions() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "foo", "2.0-1", "x86_64");

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    assert!(db_path.exists());
    let entries = list_db_entries(&db_path);
    assert_eq!(entries.len(), 1, "DB should have exactly 1 entry (latest version)");
    assert!(entries[0].contains("foo-2.0-1"), "DB should contain foo-2.0-1, got: {}", entries[0]);

    assert!(
        !repo_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Outdated package should be removed"
    );
}

#[test]
fn reset_routes_debug_packages() {
    let repo_dir = TempDir::new().unwrap();
    let debug_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let debug_db_path = debug_dir.path().join("test-debug.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), Some(debug_db_path.to_str().unwrap()));
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "foo-debug", "1.0-1", "x86_64");

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    assert_db_contains(&db_path, "foo");
    assert_db_not_contains(&db_path, "foo-debug");
    assert_db_contains(&debug_db_path, "foo-debug");

    assert!(!repo_dir.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(debug_dir.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
}

#[test]
fn reset_backs_up_outdated_packages() {
    let repo_dir = TempDir::new().unwrap();
    let backup_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = TestProfileConfig::new(TEST_PROFILE, db_path.to_str().unwrap())
        .backup(backup_dir.path().to_str().unwrap())
        .to_toml();
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "foo", "2.0-1", "x86_64");

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    let entries = list_db_entries(&db_path);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].contains("foo-2.0-1"));

    assert!(
        backup_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Outdated package should be backed up"
    );
    assert!(
        !repo_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Outdated package should be removed from repo"
    );
}

#[test]
fn update_adds_new_package() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(repo_dir.path(), "bar", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    assert_db_contains(&db_path, "foo");
    assert_db_contains(&db_path, "bar");
}

#[test]
fn update_removes_stale_package() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "bar", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    fs::remove_file(repo_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst")).unwrap();
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    assert_db_contains(&db_path, "foo");
    assert_db_not_contains(&db_path, "bar");
}

#[test]
fn update_noop_when_unchanged() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    let entries = list_db_entries(&db_path);
    assert_eq!(entries.len(), 1);
    assert_db_contains(&db_path, "foo");
}

#[test]
fn update_handles_newer_version() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(repo_dir.path(), "foo", "2.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    let entries = list_db_entries(&db_path);
    assert_eq!(entries.len(), 1, "Should have exactly 1 entry after version update");
    assert!(entries[0].contains("foo-2.0-1"), "Should have the newer version");

    assert!(
        !repo_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Old version should be cleaned up"
    );
}

#[test]
fn update_routes_debug_packages() {
    let repo_dir = TempDir::new().unwrap();
    let debug_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let debug_db_path = debug_dir.path().join("test-debug.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), Some(debug_db_path.to_str().unwrap()));
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(repo_dir.path(), "foo-debug", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    assert!(
        !repo_dir.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Debug pkg should be moved out of main repo"
    );
    assert!(
        debug_dir.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Debug pkg should be in debug dir"
    );
    assert_db_contains(&debug_db_path, "foo-debug");
    assert_db_contains(&db_path, "foo");
    assert_db_not_contains(&db_path, "foo-debug");
}

#[test]
fn move_pkgs_transfers_and_updates() {
    let repo_dir = TempDir::new().unwrap();
    let cwd_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "existing", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(cwd_dir.path(), "foo", "1.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();

    assert!(
        !cwd_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Package should be moved out of CWD"
    );
    assert!(
        repo_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Package should be in repo dir"
    );
    assert_db_contains(&db_path, "foo");
    assert_db_contains(&db_path, "existing");
}

#[test]
fn move_pkgs_noop_empty_cwd() {
    let repo_dir = TempDir::new().unwrap();
    let cwd_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    // move-pkgs-to-repo calls exclude_existing_pkgs which needs a valid DB
    create_test_pkg(repo_dir.path(), "existing", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();
}

#[test]
fn move_pkgs_excludes_existing() {
    let repo_dir = TempDir::new().unwrap();
    let cwd_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(cwd_dir.path(), "foo", "1.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();

    assert!(
        cwd_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Duplicate package should remain in CWD (excluded from move)"
    );
}

#[test]
fn move_pkgs_to_repo_routes_debug() {
    let repo_dir = TempDir::new().unwrap();
    let debug_dir = TempDir::new().unwrap();
    let cwd_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let debug_db_path = debug_dir.path().join("test-debug.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), Some(debug_db_path.to_str().unwrap()));
    let (_home, home_path) = setup_test_env(&config);

    // Include a debug package in initial reset so the debug DB gets created
    create_test_pkg(repo_dir.path(), "existing", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "existing-debug", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(cwd_dir.path(), "bar-debug", "1.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();

    assert!(!cwd_dir.path().join("bar-debug-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(
        debug_dir.path().join("bar-debug-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Debug pkg should end up in debug dir"
    );
    assert!(
        !repo_dir.path().join("bar-debug-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Debug pkg should NOT be in main repo dir"
    );
}

#[test]
fn missing_profile_fails() {
    let config = make_config("/dev/null/dummy.db.tar.zst", None);
    let (_home, home_path) = setup_test_env(&config);

    build_cmd(&home_path).args(["--profile", "nonexistent", "reset"]).assert().failure();
}

#[test]
fn missing_config_fails() {
    let home = TempDir::new().unwrap();
    build_cmd(home.path()).args(["--profile", "test", "reset"]).assert().failure();
}

#[test]
fn cleanup_backup_dir_removes_excess() {
    let repo_dir = TempDir::new().unwrap();
    let backup_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = TestProfileConfig::new(TEST_PROFILE, db_path.to_str().unwrap())
        .backup(backup_dir.path().to_str().unwrap())
        .backup_num(1)
        .to_toml();
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(backup_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(backup_dir.path(), "foo", "2.0-1", "x86_64");
    create_test_pkg(backup_dir.path(), "foo", "3.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "cleanup-backup-dir"])
        .assert()
        .success();

    let remaining: Vec<_> = fs::read_dir(backup_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().ends_with(".pkg.tar.zst"))
        .collect();
    assert_eq!(remaining.len(), 1, "Should keep only 1 version, got: {remaining:?}");
    assert!(
        backup_dir.path().join("foo-3.0-1-x86_64.pkg.tar.zst").exists(),
        "Latest version should be kept"
    );
}

#[test]
fn cleanup_backup_dir_noop_under_limit() {
    let repo_dir = TempDir::new().unwrap();
    let backup_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = TestProfileConfig::new(TEST_PROFILE, db_path.to_str().unwrap())
        .backup(backup_dir.path().to_str().unwrap())
        .backup_num(5)
        .to_toml();
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(backup_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(backup_dir.path(), "foo", "2.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "cleanup-backup-dir"])
        .assert()
        .success();

    assert!(backup_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(backup_dir.path().join("foo-2.0-1-x86_64.pkg.tar.zst").exists());
}

#[test]
fn cleanup_backup_dir_noop_when_disabled() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "cleanup-backup-dir"])
        .assert()
        .success();
}

#[test]
fn is_pkgs_up_to_date_clean_repo() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "is-pkgs-up-to-date"])
        .assert()
        .success();
}

#[test]
fn is_pkgs_up_to_date_detects_new_package() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Add bar file without updating DB — it's "brand new"
    create_test_pkg(repo_dir.path(), "bar", "1.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "is-pkgs-up-to-date"])
        .assert()
        .success();
}

#[test]
fn is_pkgs_up_to_date_detects_debug_in_prod() {
    let repo_dir = TempDir::new().unwrap();
    let debug_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let debug_db_path = debug_dir.path().join("test-debug.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), Some(debug_db_path.to_str().unwrap()));
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    create_test_pkg(repo_dir.path(), "foo-debug", "1.0-1", "x86_64");

    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "is-pkgs-up-to-date"])
        .assert()
        .success();
}

#[test]
fn move_pkgs_repo_to_repo_transfers() {
    let src_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();
    let src_db = src_dir.path().join("src.db.tar.zst");
    let dest_db = dest_dir.path().join("dest.db.tar.zst");

    let config = make_multi_profile_config(&[
        TestProfileConfig::new("src", src_db.to_str().unwrap()),
        TestProfileConfig::new("dest", dest_db.to_str().unwrap()),
    ]);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(src_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(src_dir.path(), "bar", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "src", "reset"]).assert().success();

    build_cmd(&home_path).args(["--from", "src", "--to", "dest", "move-pkgs"]).assert().success();

    assert!(dest_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(dest_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(!src_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(!src_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst").exists());

    assert_db_contains(&dest_db, "foo");
    assert_db_contains(&dest_db, "bar");
    assert_db_not_contains(&src_db, "foo");
    assert_db_not_contains(&src_db, "bar");
}

#[test]
fn move_pkgs_repo_to_repo_empty_source() {
    let src_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();
    let src_db = src_dir.path().join("src.db.tar.zst");
    let dest_db = dest_dir.path().join("dest.db.tar.zst");

    let config = make_multi_profile_config(&[
        TestProfileConfig::new("src", src_db.to_str().unwrap()),
        TestProfileConfig::new("dest", dest_db.to_str().unwrap()),
    ]);
    let (_home, home_path) = setup_test_env(&config);

    // ALPM needs a valid DB file — reset then remove the package
    create_test_pkg(src_dir.path(), "dummy", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "src", "reset"]).assert().success();
    fs::remove_file(src_dir.path().join("dummy-1.0-1-x86_64.pkg.tar.zst")).unwrap();

    build_cmd(&home_path).args(["--from", "src", "--to", "dest", "move-pkgs"]).assert().success();
}

#[test]
fn move_pkgs_repo_to_repo_preserves_dest_existing() {
    let src_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();
    let src_db = src_dir.path().join("src.db.tar.zst");
    let dest_db = dest_dir.path().join("dest.db.tar.zst");

    let config = make_multi_profile_config(&[
        TestProfileConfig::new("src", src_db.to_str().unwrap()),
        TestProfileConfig::new("dest", dest_db.to_str().unwrap()),
    ]);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(dest_dir.path(), "baz", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "dest", "reset"]).assert().success();

    create_test_pkg(src_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "src", "reset"]).assert().success();

    build_cmd(&home_path).args(["--from", "src", "--to", "dest", "move-pkgs"]).assert().success();

    assert_db_contains(&dest_db, "baz");
    assert_db_contains(&dest_db, "foo");
}

#[test]
fn sync_copies_newer_packages() {
    let profile_dir = TempDir::new().unwrap();
    let ref_dir = TempDir::new().unwrap();
    let profile_db = profile_dir.path().join("profile.db.tar.zst");
    let ref_db = ref_dir.path().join("ref.db.tar.zst");

    let config = make_multi_profile_config(&[
        TestProfileConfig::new("ref", ref_db.to_str().unwrap()),
        TestProfileConfig::new(TEST_PROFILE, profile_db.to_str().unwrap())
            .reference_repo(ref_db.to_str().unwrap()),
    ]);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(ref_dir.path(), "foo", "2.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "ref", "reset"]).assert().success();

    create_test_pkg(profile_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "sync"]).assert().success();

    assert!(
        profile_dir.path().join("foo-2.0-1-x86_64.pkg.tar.zst").exists(),
        "Newer package from ref repo should be copied to profile dir"
    );
}

#[test]
fn sync_noop_when_up_to_date() {
    let profile_dir = TempDir::new().unwrap();
    let ref_dir = TempDir::new().unwrap();
    let profile_db = profile_dir.path().join("profile.db.tar.zst");
    let ref_db = ref_dir.path().join("ref.db.tar.zst");

    let config = make_multi_profile_config(&[
        TestProfileConfig::new("ref", ref_db.to_str().unwrap()),
        TestProfileConfig::new(TEST_PROFILE, profile_db.to_str().unwrap())
            .reference_repo(ref_db.to_str().unwrap()),
    ]);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(ref_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "ref", "reset"]).assert().success();

    create_test_pkg(profile_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    let mut files_before: Vec<_> = fs::read_dir(profile_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_str().unwrap().to_string())
        .collect();
    files_before.sort();

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "sync"]).assert().success();

    let mut files_after: Vec<_> = fs::read_dir(profile_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_str().unwrap().to_string())
        .collect();
    files_after.sort();

    assert_eq!(files_before, files_after, "No files should change when up to date");
}

#[test]
fn sync_routes_debug_packages() {
    let profile_dir = TempDir::new().unwrap();
    let ref_dir = TempDir::new().unwrap();
    let debug_dir = TempDir::new().unwrap();
    let profile_db = profile_dir.path().join("profile.db.tar.zst");
    let ref_db = ref_dir.path().join("ref.db.tar.zst");
    let debug_db = debug_dir.path().join("profile-debug.db.tar.zst");

    // Set up profile WITHOUT debug_repo first, so foo-debug-1.0 stays in main DB
    let initial_config = make_multi_profile_config(&[
        TestProfileConfig::new("ref", ref_db.to_str().unwrap()),
        TestProfileConfig::new(TEST_PROFILE, profile_db.to_str().unwrap())
            .reference_repo(ref_db.to_str().unwrap()),
    ]);
    let (_home, home_path) = setup_test_env(&initial_config);

    create_test_pkg(ref_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(ref_dir.path(), "foo-debug", "2.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", "ref", "reset"]).assert().success();

    create_test_pkg(profile_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(profile_dir.path(), "foo-debug", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Rewrite config with debug_repo enabled for the sync step
    let final_config = make_multi_profile_config(&[
        TestProfileConfig::new("ref", ref_db.to_str().unwrap()),
        TestProfileConfig::new(TEST_PROFILE, profile_db.to_str().unwrap())
            .reference_repo(ref_db.to_str().unwrap())
            .debug_repo(debug_db.to_str().unwrap()),
    ]);
    rewrite_config(&home_path, &final_config);

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "sync"]).assert().success();

    assert!(
        debug_dir.path().join("foo-debug-2.0-1-x86_64.pkg.tar.zst").exists(),
        "Debug pkg should be in debug dir"
    );
    assert!(
        !profile_dir.path().join("foo-debug-2.0-1-x86_64.pkg.tar.zst").exists(),
        "Debug pkg should NOT be in main profile dir"
    );
}

#[test]
fn sync_skips_without_reference_repo() {
    let profile_dir = TempDir::new().unwrap();
    let db_path = profile_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(profile_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "sync"]).assert().success();
}
