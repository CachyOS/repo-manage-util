mod common;

use std::ops::Deref;
use std::path::PathBuf;

use pg_impl::db::Db;
use repo_manage_util::pkg_utils::get_repo_db_prefix;
use tempfile::TempDir;

use common::*;

fn get_database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty())
}

macro_rules! require_pg {
    () => {
        match get_database_url() {
            Some(url) => url,
            None => {
                panic!("DATABASE_URL not set");
            },
        }
    };
}

/// RAII guard that cleans up PG repo data.
// TODO(vnepogodin): refactor me later
struct PgRepoGuard {
    url: String,
    repo_names: Vec<String>,
}

impl PgRepoGuard {
    fn new(url: String, repo_names: Vec<String>) -> Self {
        Self { url, repo_names }
    }
}

impl Drop for PgRepoGuard {
    fn drop(&mut self) {
        let url = self.url.clone();
        let repo_names = self.repo_names.clone();
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let db = match Db::connect(&url).await {
                    Ok(db) => db,
                    Err(_) => return,
                };
                if db.migrate().await.is_err() {
                    return;
                }
                for name in &repo_names {
                    let _ = db.remove_existing_repository(name).await;
                }
            });
        })
        .join();
    }
}

/// Shared test context
struct PgTestCtx {
    home: TempDir,
    home_path: PathBuf,
    repo_dir: TempDir,
    db_path: PathBuf,
    repo_name: String,
    db: Db,
    // kept alive for cleanup-on-drop
    #[allow(dead_code)]
    guard: PgRepoGuard,
}

impl PgTestCtx {
    async fn new(pg_url: &str, profile_name: &str, db_suffix: &str) -> Self {
        let repo_dir = TempDir::new().unwrap();
        let db_path = repo_dir.path().join(format!("{db_suffix}.db.tar.zst"));
        let repo_name = get_repo_db_prefix(db_path.to_str().unwrap());

        let db = Db::connect(pg_url).await.unwrap();
        db.migrate().await.unwrap();

        let guard = PgRepoGuard::new(pg_url.to_string(), vec![repo_name.clone()]);

        let config = make_pg_config(pg_url, profile_name, db_path.to_str().unwrap());
        let (home, home_path) = setup_test_env(&config);

        Self {
            home,
            home_path,
            repo_dir,
            db_path,
            repo_name,
            db,
            guard,
        }
    }

    /// Build a CLI command for this test context.
    fn cmd(&self) -> assert_cmd::Command {
        build_cmd(&self.home_path)
    }
}

impl Deref for PgTestCtx {
    type Target = Db;
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

fn make_pg_config(pg_url: &str, profile_name: &str, repo_db_path: &str) -> String {
    let mut s = format!("postgresql_url = \"{pg_url}\"\n\n");
    s.push_str(&TestProfileConfig::new(profile_name, repo_db_path).to_toml());
    s
}

fn make_multi_pg_config(pg_url: &str, profiles: &[TestProfileConfig<'_>]) -> String {
    let mut s = format!("postgresql_url = \"{pg_url}\"\n\n");
    for p in profiles {
        s.push_str(&p.to_toml());
        s.push('\n');
    }
    s
}

async fn pg_package_names(db: &Db, repo_name: &str) -> Vec<String> {
    db.get_repo_packages(repo_name)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|p| p.pkg_name)
        .collect()
}

async fn assert_pg_has(db: &Db, repo_name: &str, pkg_name: &str) {
    let names = pg_package_names(db, repo_name).await;
    assert!(
        names.iter().any(|n| n == pkg_name),
        "PG repo '{repo_name}' should contain '{pkg_name}', got: {names:?}"
    );
}

async fn assert_pg_not_has(db: &Db, repo_name: &str, pkg_name: &str) {
    let names = pg_package_names(db, repo_name).await;
    assert!(
        !names.iter().any(|n| n == pkg_name),
        "PG repo '{repo_name}' should NOT contain '{pkg_name}', got: {names:?}"
    );
}

async fn assert_pg_version(db: &Db, repo_name: &str, pkg_name: &str, expected: &str) {
    let pkg = db.get_package_info(repo_name, pkg_name).await.unwrap();
    assert!(pkg.is_some(), "PG repo '{repo_name}' should contain '{pkg_name}'");
    assert_eq!(
        pkg.unwrap().pkg_version.as_deref(),
        Some(expected),
        "PG package '{pkg_name}' version mismatch"
    );
}

#[tokio::test]
async fn reset_populates_pg() {
    let pg_url = require_pg!();
    let ctx = PgTestCtx::new(&pg_url, "test", "pg-reset-test").await;

    create_test_pkg(&ctx.repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(&ctx.repo_dir.path(), "bar", "2.0-1", "x86_64");

    ctx.cmd()
        .args(["--profile", "test", "reset"])
        .assert()
        .success();

    assert!(ctx.db_path.exists());
    assert_pg_has(&ctx.db, &ctx.repo_name, "foo").await;
    assert_pg_has(&ctx.db, &ctx.repo_name, "bar").await;
    assert_pg_version(&ctx.db, &ctx.repo_name, "foo", "1.0-1").await;
    assert_pg_version(&ctx.db, &ctx.repo_name, "bar", "2.0-1").await;
}

#[tokio::test]
async fn reset_only_pg_skips_files() {
    let pg_url = require_pg!();
    let ctx = PgTestCtx::new(&pg_url, "test", "pg-onlypg-test").await;

    create_test_pkg(&ctx.repo_dir.path(), "foo", "1.0-1", "x86_64");

    // Normal reset — creates .db.tar.zst and populates PG.
    ctx.cmd()
        .args(["--profile", "test", "reset"])
        .assert()
        .success();

    assert!(ctx.db_path.exists());

    // --only-pg reset: file operations skipped, .db.tar.zst preserved.
    ctx.cmd()
        .args(["--profile", "test", "--only-pg", "reset"])
        .assert()
        .success();

    assert!(ctx.db_path.exists(), "DB file should survive --only-pg reset");
    assert_pg_has(&ctx.db, &ctx.repo_name, "foo").await;
}

#[tokio::test]
async fn reset_only_pg_idempotent() {
    let pg_url = require_pg!();
    let ctx = PgTestCtx::new(&pg_url, "test", "pg-idem-test").await;

    create_test_pkg(&ctx.repo_dir.path(), "foo", "1.0-1", "x86_64");
    ctx.cmd()
        .args(["--profile", "test", "reset"])
        .assert()
        .success();

    assert_pg_has(&ctx.db, &ctx.repo_name, "foo").await;

    // Re-run --only-pg: no new packages, PG still has foo.
    ctx.cmd()
        .args(["--profile", "test", "--only-pg", "reset"])
        .assert()
        .success();

    assert_pg_has(&ctx.db, &ctx.repo_name, "foo").await;
}

#[tokio::test]
async fn update_adds_and_removes_pg_entries() {
    let pg_url = require_pg!();
    let ctx = PgTestCtx::new(&pg_url, "test", "pg-update-test").await;

    create_test_pkg(&ctx.repo_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(&ctx.repo_dir.path(), "bar", "1.0-1", "x86_64");
    ctx.cmd()
        .args(["--profile", "test", "reset"])
        .assert()
        .success();

    // Add baz, remove bar, then update.
    create_test_pkg(&ctx.repo_dir.path(), "baz", "1.0-1", "x86_64");
    std::fs::remove_file(ctx.repo_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst")).unwrap();

    ctx.cmd()
        .args(["--profile", "test", "update"])
        .assert()
        .success();

    assert_pg_has(&ctx.db, &ctx.repo_name, "foo").await;
    assert_pg_has(&ctx.db, &ctx.repo_name, "baz").await;
    assert_pg_not_has(&ctx.db, &ctx.repo_name, "bar").await;
}

#[tokio::test]
async fn update_version_bump_pg() {
    let pg_url = require_pg!();
    let ctx = PgTestCtx::new(&pg_url, "test", "pg-ver-test").await;

    create_test_pkg(&ctx.repo_dir.path(), "foo", "1.0-1", "x86_64");
    ctx.cmd()
        .args(["--profile", "test", "reset"])
        .assert()
        .success();

    assert_pg_version(&ctx.db, &ctx.repo_name, "foo", "1.0-1").await;

    create_test_pkg(&ctx.repo_dir.path(), "foo", "2.0-1", "x86_64");
    ctx.cmd()
        .args(["--profile", "test", "update"])
        .assert()
        .success();

    assert_pg_version(&ctx.db, &ctx.repo_name, "foo", "2.0-1").await;
}

#[tokio::test]
async fn move_pkgs_to_repo_updates_pg() {
    let pg_url = require_pg!();
    let ctx = PgTestCtx::new(&pg_url, "test", "pg-moveto-test").await;
    let cwd_dir = TempDir::new().unwrap();

    create_test_pkg(&ctx.repo_dir.path(), "existing", "1.0-1", "x86_64");
    ctx.cmd()
        .args(["--profile", "test", "reset"])
        .assert()
        .success();

    create_test_pkg(cwd_dir.path(), "newcomer", "1.0-1", "x86_64");

    ctx.cmd()
        .args(["--profile", "test", "move-pkgs-to-repo"])
        .current_dir(cwd_dir.path())
        .assert()
        .success();

    assert_pg_has(&ctx.db, &ctx.repo_name, "existing").await;
    assert_pg_has(&ctx.db, &ctx.repo_name, "newcomer").await;
}

#[tokio::test]
async fn move_pkgs_repo_to_repo_updates_pg() {
    let pg_url = require_pg!();

    let src_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();
    let src_db_path = src_dir.path().join("pg-mv-src.db.tar.zst");
    let dest_db_path = dest_dir.path().join("pg-mv-dest.db.tar.zst");
    let src_repo = get_repo_db_prefix(src_db_path.to_str().unwrap());
    let dest_repo = get_repo_db_prefix(dest_db_path.to_str().unwrap());

    let db = Db::connect(&pg_url).await.unwrap();
    db.migrate().await.unwrap();
    let _guard = PgRepoGuard::new(pg_url.clone(), vec![src_repo.clone(), dest_repo.clone()]);

    let config = make_multi_pg_config(
        &pg_url,
        &[
            TestProfileConfig::new("src", src_db_path.to_str().unwrap()),
            TestProfileConfig::new("dest", dest_db_path.to_str().unwrap()),
        ],
    );
    let (_home, home_path) = setup_test_env(&config);

    create_test_pkg(dest_dir.path(), "baz", "1.0-1", "x86_64");
    build_cmd(&home_path)
        .args(["--profile", "dest", "reset"])
        .assert()
        .success();

    assert_pg_has(&db, &dest_repo, "baz").await;

    create_test_pkg(src_dir.path(), "foo", "1.0-1", "x86_64");
    create_test_pkg(src_dir.path(), "bar", "1.0-1", "x86_64");
    build_cmd(&home_path)
        .args(["--profile", "src", "reset"])
        .assert()
        .success();

    assert_pg_has(&db, &src_repo, "foo").await;
    assert_pg_has(&db, &src_repo, "bar").await;

    build_cmd(&home_path)
        .args(["--from", "src", "--to", "dest", "move-pkgs"])
        .assert()
        .success();

    assert_pg_not_has(&db, &src_repo, "foo").await;
    assert_pg_not_has(&db, &src_repo, "bar").await;

    assert_pg_has(&db, &dest_repo, "foo").await;
    assert_pg_has(&db, &dest_repo, "bar").await;
    assert_pg_has(&db, &dest_repo, "baz").await;

    assert!(dest_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(dest_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(!src_dir.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
    assert!(!src_dir.path().join("bar-1.0-1-x86_64.pkg.tar.zst").exists());
}
