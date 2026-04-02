/// Shared test helpers for integration tests.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

pub const TEST_PROFILE: &str = "test";

pub fn create_test_pkg(dir: &Path, name: &str, version: &str, arch: &str) -> PathBuf {
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

pub fn setup_test_env(config_content: &str) -> (TempDir, PathBuf) {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/repo-manage");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), config_content).unwrap();
    let home_path = home.path().to_path_buf();
    (home, home_path)
}

pub fn build_cmd(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("repo-manage-util").unwrap();
    cmd.env("HOME", home);
    // Propagate DATABASE_URL when available (no-op for non-PG tests).
    if let Ok(url) = std::env::var("DATABASE_URL") {
        cmd.env("DATABASE_URL", url);
    }
    cmd
}

pub fn rewrite_config(home: &Path, config: &str) {
    fs::write(home.join(".config/repo-manage/config.toml"), config).unwrap();
}

pub struct TestProfileConfig<'a> {
    pub name: &'a str,
    pub repo: &'a str,
    pub debug_repo: Option<&'a str>,
    pub backup_dir: Option<&'a str>,
    pub backup_num: Option<usize>,
    pub reference_repo: Option<&'a str>,
}

impl<'a> TestProfileConfig<'a> {
    pub fn new(name: &'a str, repo: &'a str) -> Self {
        Self {
            name,
            repo,
            debug_repo: None,
            backup_dir: None,
            backup_num: None,
            reference_repo: None,
        }
    }

    pub fn debug_repo(mut self, path: &'a str) -> Self {
        self.debug_repo = Some(path);
        self
    }

    pub fn backup(mut self, backup_dir: &'a str) -> Self {
        self.backup_dir = Some(backup_dir);
        self
    }

    pub fn backup_num(mut self, n: usize) -> Self {
        self.backup_num = Some(n);
        self
    }

    pub fn reference_repo(mut self, path: &'a str) -> Self {
        self.reference_repo = Some(path);
        self
    }

    pub fn to_toml(&self) -> String {
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

pub fn make_multi_profile_config(profiles: &[TestProfileConfig<'_>]) -> String {
    profiles.iter().map(|p| p.to_toml()).collect::<Vec<_>>().join("\n")
}

pub fn make_config(repo_db_path: &str, debug_db_path: Option<&str>) -> String {
    let mut cfg = TestProfileConfig::new(TEST_PROFILE, repo_db_path);
    if let Some(d) = debug_db_path {
        cfg = cfg.debug_repo(d);
    }
    cfg.to_toml()
}

pub fn list_db_entries(db_path: &Path) -> Vec<String> {
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

pub fn db_entry_names(entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .map(|e| {
            let pos = e.match_indices('-').nth_back(1).map(|(i, _)| i).unwrap_or(e.len());
            e[..pos].to_string()
        })
        .collect()
}

#[track_caller]
pub fn assert_db_contains(db_path: &Path, pkg_name: &str) {
    let names = db_entry_names(&list_db_entries(db_path));
    assert!(
        names.contains(&pkg_name.to_string()),
        "DB should contain '{pkg_name}', got: {names:?}"
    );
}

#[track_caller]
pub fn assert_db_not_contains(db_path: &Path, pkg_name: &str) {
    let names = db_entry_names(&list_db_entries(db_path));
    assert!(
        !names.contains(&pkg_name.to_string()),
        "DB should NOT contain '{pkg_name}', got: {names:?}"
    );
}

