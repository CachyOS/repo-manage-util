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

/// Flexible config builder for test profiles.
struct TestProfileConfig<'a> {
    name: &'a str,
    repo: &'a str,
    debug_repo: Option<&'a str>,
    backup_dir: Option<&'a str>,
    backup_num: Option<usize>,
    reference_repo: Option<&'a str>,
}

impl<'a> TestProfileConfig<'a> {
    fn new(name: &'a str, repo: &'a str) -> Self {
        Self {
            name,
            repo,
            debug_repo: None,
            backup_dir: None,
            backup_num: None,
            reference_repo: None,
        }
    }

    fn debug_repo(mut self, path: &'a str) -> Self {
        self.debug_repo = Some(path);
        self
    }

    fn backup(mut self, backup_dir: &'a str) -> Self {
        self.backup_dir = Some(backup_dir);
        self
    }

    fn backup_num(mut self, n: usize) -> Self {
        self.backup_num = Some(n);
        self
    }

    fn reference_repo(mut self, path: &'a str) -> Self {
        self.reference_repo = Some(path);
        self
    }

    fn to_toml(&self) -> String {
        let backup = self.backup_dir.is_some();
        let mut s = format!(
            "[profiles.{}]\nrepo = \"{}\"\nadd_params = []\nrm_params = []\nrequire_signature = \
             false\nbackup = {}\n",
            self.name, self.repo, backup
        );
        if let Some(d) = self.debug_repo {
            s.push_str(&format!("debug_repo = \"{d}\"\n"));
        }
        if let Some(d) = self.backup_dir {
            s.push_str(&format!("backup_dir = \"{d}\"\n"));
        }
        if let Some(n) = self.backup_num {
            s.push_str(&format!("backup_num = {n}\n"));
        }
        if let Some(r) = self.reference_repo {
            s.push_str(&format!("reference_repo = \"{r}\"\n"));
        }
        s
    }
}

fn make_multi_profile_config(profiles: &[TestProfileConfig<'_>]) -> String {
    profiles.iter().map(|p| p.to_toml()).collect::<Vec<_>>().join("\n")
}

/// Generate a config TOML string, optionally with a debug repo.
fn make_config(repo_db_path: &str, debug_db_path: Option<&str>) -> String {
    let mut cfg = TestProfileConfig::new(TEST_PROFILE, repo_db_path);
    if let Some(d) = debug_db_path {
        cfg = cfg.debug_repo(d);
    }
    cfg.to_toml()
}

/// Rewrite the config file in a test HOME directory.
fn rewrite_config(home: &Path, config: &str) {
    fs::write(home.join(".config/repo-manage/config.toml"), config).unwrap();
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
            // last 2 dashes delimit ver-rel
            let pos = e.match_indices('-').nth_back(1).map(|(i, _)| i).unwrap_or(e.len());
            e[..pos].to_string()
        })
        .collect()
}

/// Assert that a repo DB contains a package with the given name.
#[track_caller]
fn assert_db_contains(db_path: &Path, pkg_name: &str) {
    let names = db_entry_names(&list_db_entries(db_path));
    assert!(
        names.contains(&pkg_name.to_string()),
        "DB should contain '{pkg_name}', got: {names:?}"
    );
}

/// Assert that a repo DB does NOT contain a package with the given name.
#[track_caller]
fn assert_db_not_contains(db_path: &Path, pkg_name: &str) {
    let names = db_entry_names(&list_db_entries(db_path));
    assert!(
        !names.contains(&pkg_name.to_string()),
        "DB should NOT contain '{pkg_name}', got: {names:?}"
    );
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
