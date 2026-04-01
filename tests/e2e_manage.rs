use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

const TEST_PROFILE: &str = "test";

/// Build a valid `.pkg.tar.zst` file containing a `.PKGINFO`.
/// Returns the full path to the created package file.
fn create_test_pkg(dir: &Path, name: &str, version: &str, arch: &str) -> PathBuf {
    let filename = format!("{name}-{version}-{arch}.pkg.tar.zst");
    let dest = dir.join(&filename);

    let pkginfo = format!(
        "pkgname = {name}\n\
         pkgbase = {name}\n\
         pkgver = {version}\n\
         pkgdesc = Test package {name}\n\
         url = https://example.com\n\
         builddate = 1700000000\n\
         packager = Test <test@test.com>\n\
         size = 0\n\
         arch = {arch}\n"
    );

    // tar -> zstd
    let zstd_file = fs::File::create(&dest).unwrap();
    let zstd_enc = zstd::Encoder::new(zstd_file, 1).unwrap();
    let mut tar_builder = tar::Builder::new(zstd_enc);

    let pkginfo_bytes = pkginfo.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_path(".PKGINFO").unwrap();
    header.set_size(pkginfo_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar_builder.append(&header, pkginfo_bytes).unwrap();

    let zstd_enc = tar_builder.into_inner().unwrap();
    zstd_enc.finish().unwrap();

    dest
}

/// Create a temp HOME directory with a config file.
/// Returns the TempDir (must be kept alive) and the home path.
fn setup_test_env(config_content: &str) -> (TempDir, PathBuf) {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/repo-manage");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();
    let home_path = home.path().to_path_buf();
    (home, home_path)
}

/// Build an assert_cmd Command for the binary with HOME overridden.
fn build_cmd(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("repo-manage-util").unwrap();
    cmd.env("HOME", home);
    cmd
}

/// Generate a config TOML string, optionally with a debug repo.
fn make_config(repo_db_path: &str, debug_db_path: Option<&str>) -> String {
    let mut config = format!(
        "[profiles.{TEST_PROFILE}]\nrepo = \"{repo_db_path}\"\nadd_params = []\nrm_params = \
         []\nrequire_signature = false\nbackup = false\n"
    );
    if let Some(debug_path) = debug_db_path {
        config.push_str(&format!("debug_repo = \"{debug_path}\"\n"));
    }
    config
}

/// List package entries from a repo DB file (`.db.tar.zst`).
/// Returns sorted list of `{pkgname}-{pkgver}` strings.
fn list_db_entries(db_path: &Path) -> Vec<String> {
    if !db_path.exists() {
        return vec![];
    }
    let file = fs::File::open(db_path).unwrap();
    let decoder = zstd::Decoder::new(file).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let mut entries = HashSet::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap();
        // Top-level dirs look like "pkgname-pkgver/"
        if let Some(first_component) = path.components().next() {
            let name = first_component.as_os_str().to_str().unwrap().to_string();
            entries.insert(name);
        }
    }
    let mut sorted: Vec<String> = entries.into_iter().collect();
    sorted.sort();
    sorted
}

/// Extract just package names (without version) from DB entries.
fn db_entry_names(entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .map(|e| {
            // Entry format: "pkgname-pkgver" where pkgver is "ver-rel"
            // Use the same logic as pkg_utils: last 2 dashes delimit ver-rel
            let pos = e.match_indices('-').nth_back(1).map(|(i, _)| i).unwrap_or(e.len());
            e[..pos].to_string()
        })
        .collect()
}

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
    let entries = list_db_entries(&db_path);
    let names = db_entry_names(&entries);
    assert!(names.contains(&"foo".to_string()), "DB should contain foo");
    assert!(names.contains(&"bar".to_string()), "DB should contain bar");
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

    // Outdated package file should be deleted
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

    // Main DB should contain foo only
    let main_entries = db_entry_names(&list_db_entries(&db_path));
    assert!(main_entries.contains(&"foo".to_string()));
    assert!(!main_entries.contains(&"foo-debug".to_string()));

    // Debug DB should contain foo-debug
    let debug_entries = db_entry_names(&list_db_entries(&debug_db_path));
    assert!(
        debug_entries.contains(&"foo-debug".to_string()),
        "Debug DB should contain foo-debug, got: {:?}",
        debug_entries
    );

    // Debug package should have been moved out of main repo dir
    assert!(!repo_dir.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
    // Debug package should exist in debug dir
    assert!(debug_dir.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
}

#[test]
fn update_adds_new_package() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    // Initial reset with foo
    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Add bar, then update
    create_test_pkg(repo_dir.path(), "bar", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    let names = db_entry_names(&list_db_entries(&db_path));
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"bar".to_string()));
}

#[test]
fn update_removes_stale_package() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    // Initial reset with foo + bar
    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(repo_dir.path(), "bar", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Remove bar file, then update
    fs::remove_file(repo_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst")).unwrap();
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    let names = db_entry_names(&list_db_entries(&db_path));
    assert!(names.contains(&"foo".to_string()));
    assert!(!names.contains(&"bar".to_string()), "bar should be removed as stale");
}

#[test]
fn update_noop_when_unchanged() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Update with no changes
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    let entries = list_db_entries(&db_path);
    assert_eq!(entries.len(), 1);
    let names = db_entry_names(&entries);
    assert!(names.contains(&"foo".to_string()));
}

#[test]
fn update_handles_newer_version() {
    let repo_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    // Initial reset with foo-1.0-1
    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Add newer version
    create_test_pkg(repo_dir.path(), "foo", "2.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "update"]).assert().success();

    let entries = list_db_entries(&db_path);
    assert_eq!(entries.len(), 1, "Should have exactly 1 entry after version update");
    assert!(entries[0].contains("foo-2.0-1"), "Should have the newer version");

    // Old version file should be deleted
    assert!(
        !repo_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Old version should be cleaned up"
    );
}

#[test]
fn move_pkgs_transfers_and_updates() {
    let repo_dir = TempDir::new().unwrap();
    let cwd_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    // Reset repo with an existing package so the ALPM DB is valid
    create_test_pkg(repo_dir.path(), "existing", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Create new package in CWD
    create_test_pkg(cwd_dir.path(), "foo", "1.0-1", "x86_64");

    // Move packages to repo
    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();

    // Package should be moved out of CWD
    assert!(
        !cwd_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Package should be moved out of CWD"
    );
    // Package should be in repo dir
    assert!(
        repo_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Package should be in repo dir"
    );
    // DB should contain both existing and foo
    let names = db_entry_names(&list_db_entries(&db_path));
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"existing".to_string()));
}

#[test]
fn move_pkgs_noop_empty_cwd() {
    let repo_dir = TempDir::new().unwrap();
    let cwd_dir = TempDir::new().unwrap();
    let db_path = repo_dir.path().join("test.db.tar.zst");
    let config = make_config(db_path.to_str().unwrap(), None);
    let (_home, home_path) = setup_test_env(&config);

    // Reset repo with a package so the ALPM DB is valid
    // (move-pkgs-to-repo always calls exclude_existing_pkgs which needs a valid DB)
    create_test_pkg(repo_dir.path(), "existing", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Move with empty CWD
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

    // Reset with foo already in repo
    create_test_pkg(repo_dir.path(), "foo", "1.0-1", "x86_64");
    build_cmd(&home_path).args(["--profile", TEST_PROFILE, "reset"]).assert().success();

    // Create same package in CWD
    create_test_pkg(cwd_dir.path(), "foo", "1.0-1", "x86_64");

    // Move should exclude the duplicate
    build_cmd(&home_path)
        .args(["--profile", TEST_PROFILE, "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();

    // CWD file should still be there (excluded, not moved)
    assert!(
        cwd_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists(),
        "Duplicate package should remain in CWD (excluded from move)"
    );
}
