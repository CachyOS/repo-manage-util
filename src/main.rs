use repo_manage_util::{
    alpm_helper, args, aur, config, logger, pkg_utils, postgresql_helper, repo_utils,
};
mod rebuild_check;

use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result};
use args::{Cli, Commands};
use clap::Parser;
use config::Profile;
use postgresql_helper::PostgresqlHelper;

fn get_profile_from_config<'a>(
    profile_name: &'a str,
    config: &'a config::Config,
) -> Result<&'a config::Profile> {
    config.profiles.get(profile_name).ok_or(anyhow::anyhow!("Profile {profile_name} not found"))
}

fn get_repo_dir_from_profile(profile: &config::Profile) -> &Path {
    Path::new(&profile.repo).parent().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // initialize the logger
    logger::init_logger();

    // load config
    let config_path = config::get_config_path()?;
    let config = config::parse_config_file(&config_path)?;

    // Initialize PostgreSQL helper if URL is provided
    let pg_helper = if let Some(postgresql_url) = &config.postgresql_url {
        let helper = PostgresqlHelper::new(postgresql_url).await?;
        Some(helper)
    } else {
        None
    };

    match &args.command {
        Commands::Reset(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            let repo_db_prefix = pkg_utils::get_repo_db_prefix(&profile.repo);
            let repo_db_pattern = format!("{}/{repo_db_prefix}.*", repo_dir.to_str().unwrap());

            tracing::debug!("repo db path := {repo_db_pattern}");

            do_repo_reset(profile, &repo_db_pattern, repo_dir, pg_helper, args.only_pg).await?;
        },
        Commands::Update(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_update(profile, repo_dir, pg_helper).await?;
        },
        Commands::Sync(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_sync(profile, repo_dir).await?;
        },
        Commands::MovePkgsToRepo(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_move_pkgs(profile, repo_dir, pg_helper).await?;
        },
        Commands::IsPkgsUpToDate(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_checkup(profile, repo_dir).await?;
        },
        Commands::CleanupBackupDir(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;

            do_backup_repo_cleanup(profile)?;
        },
        Commands::MovePkgs(args) => {
            let from_profile = get_profile_from_config(&args.from, &config)?;
            let from_repo_dir = get_repo_dir_from_profile(from_profile);

            let to_profile = get_profile_from_config(&args.to, &config)?;
            let to_repo_dir = get_repo_dir_from_profile(to_profile);

            move_packages_from_repo_to_repo(
                from_profile,
                from_repo_dir,
                to_profile,
                to_repo_dir,
                pg_helper,
            )
            .await?;
        },
        Commands::Aur(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_aur(repo_dir, args.order_file.clone(), args.dry_run).await?;
        },
        Commands::CheckRebuild(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_check_rebuild(repo_dir, args.pacman_conf.as_deref())?;
        },
    }

    Ok(())
}

async fn do_repo_reset(
    profile: &config::Profile,
    repo_db_pattern: &str,
    repo_dir: &Path,
    pg_helper: Option<PostgresqlHelper>,
    only_pg: bool,
) -> Result<()> {
    if !only_pg {
        // Remove db and files
        for pattern in [repo_db_pattern] {
            tracing::debug!("removing db file '{pattern}'..");
            for entry in glob::glob(pattern)? {
                fs::remove_file(entry?)?;
            }
        }

        // Also wipe debug repo DB so it gets rebuilt from current debug packages
        if let Some(ref debug_repo) = profile.debug_repo
            && debug_repo != &profile.repo
        {
            let debug_dir = Path::new(debug_repo).parent().unwrap();
            let debug_db_prefix = pkg_utils::get_repo_db_prefix(debug_repo);
            let debug_db_pattern = format!("{}/{debug_db_prefix}.*", debug_dir.to_str().unwrap());
            for entry in glob::glob(&debug_db_pattern)? {
                fs::remove_file(entry?)?;
            }
        }

        let mut pkgs_list = pkg_utils::find_packages_in_dir(repo_dir)?;
        let outdated_pkgs = pkg_utils::get_outdated_pkgs(&pkgs_list);
        pkgs_list.retain(|pkg| !outdated_pkgs.contains(pkg));

        // don't insert packages without signature
        if profile.require_signature {
            pkg_utils::remove_pkgs_without_sig(&mut pkgs_list);
        }

        let debug_pkgs = if profile.debug_repo.is_some() {
            pkg_utils::exclude_debug_pkgs(&mut pkgs_list)
        } else {
            vec![]
        };

        // run repo-add
        repo_utils::handle_repo_add(profile, &pkgs_list)?;

        // move debug pkgs from prod to debug dir, then rebuild debug DB from all debug dir contents
        if let Some(ref debug_repo) = profile.debug_repo
            && debug_repo != &profile.repo
        {
            let debug_dir = Path::new(debug_repo).parent().unwrap();
            if !debug_pkgs.is_empty() {
                fs::create_dir_all(debug_dir)?;
                handle_pkgfiles_move(&debug_pkgs, debug_dir.to_str().unwrap())?;
            }
            if debug_dir.exists() {
                let all_debug = pkg_utils::find_packages_in_dir(debug_dir)?;
                if !all_debug.is_empty() {
                    repo_utils::repo_add(debug_repo, &profile.add_params, &all_debug)?;
                }
            }
        }

        // handle removal/backup here
        handle_outdated_pkgs(profile, &outdated_pkgs)?;
    }

    // Reset db if configured
    if let Some(ref pg) = pg_helper {
        // purge repo packages first then populate
        let mut tx = pg.begin().await?;
        let repo_name = pkg_utils::get_repo_db_prefix(&profile.repo);
        PostgresqlHelper::remove_existing_repository_on(&mut tx, &repo_name).await?;
        alpm_helper::populate_repo_to_db(&profile.repo, &mut tx).await?;
        tx.commit().await.context("Failed to commit repo reset transaction")?;
    }

    tracing::info!("Repo reset is done!");

    Ok(())
}

async fn do_repo_update(
    profile: &config::Profile,
    repo_dir: &Path,
    pg_helper: Option<PostgresqlHelper>,
) -> Result<()> {
    let pkgs_list = pkg_utils::find_packages_in_dir(repo_dir)?;
    let outdated_pkgs = pkg_utils::get_outdated_pkgs(&pkgs_list);
    let mut new_pkgs = pkg_utils::get_new_pkgs(&pkgs_list);

    // 1. handle new packages

    // handle new packages which are not present in the DB
    let mut brand_new_pkgs = alpm_helper::get_brand_new_packages(&profile.repo)
        .context("Failed to get brand new pkgs")?;
    // after append the brand_new_pkgs becomes invalidated (e.g empty Vec)
    new_pkgs.append(&mut brand_new_pkgs);

    // don't insert packages without signature
    if profile.require_signature {
        pkg_utils::remove_pkgs_without_sig(&mut new_pkgs);
    }

    let debug_pkgs = if profile.debug_repo.is_some() {
        pkg_utils::exclude_debug_pkgs(&mut new_pkgs)
    } else {
        vec![]
    };

    // if update available then update the DB accordingly
    // overwise silently skip and go to stale packages handling
    if !new_pkgs.is_empty() {
        // TODO(vnepogodin): print which new packages we add
        // e.g adding new package 'pacman'..

        repo_utils::handle_repo_add(profile, &new_pkgs)?;

        // 1.1 handle removal/backup of old packages here
        // NOTE: we are likely to handle it equally for update and reset. lets hope so?
        handle_outdated_pkgs(profile, &outdated_pkgs)?;
    }

    route_debug_pkgs_to_debug_repo(profile, &debug_pkgs, false)?;

    // 2. handle stale packages
    let stale_pkgs =
        alpm_helper::get_stale_packages(&profile.repo).context("Failed to get stale pkgs")?;

    // if we found stale packages then remove them from DB
    // overwise silently skip and finish update command
    if !stale_pkgs.is_empty() {
        repo_utils::handle_repo_remove(profile, &stale_pkgs)?;
    }

    // remove stale packages from debug repo DB
    if let Some(ref debug_repo) = profile.debug_repo
        && debug_repo != &profile.repo
    {
        let stale_debug = alpm_helper::get_stale_packages(debug_repo)
            .context("Failed to get stale debug pkgs")?;
        if !stale_debug.is_empty() {
            repo_utils::repo_remove(debug_repo, &profile.rm_params, &stale_debug)?;
        }
    }

    // report status only when had some work
    if !new_pkgs.is_empty() || !stale_pkgs.is_empty() {
        // Update db if configured
        if let Some(ref pg) = pg_helper {
            let mut tx = pg.begin().await?;
            let repo_name = pkg_utils::get_repo_db_prefix(&profile.repo);
            PostgresqlHelper::remove_packages_on(&mut tx, &repo_name, &stale_pkgs).await?;
            alpm_helper::add_pkgs_to_db(&profile.repo, &mut tx, &new_pkgs).await?;
            tx.commit().await.context("Failed to commit repo update transaction")?;
        }

        tracing::info!("Repo update is done!");
    } else {
        tracing::info!("nothing to do");
    }

    Ok(())
}

async fn do_repo_sync(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    if profile.reference_repo.is_none() {
        tracing::error!("Reference repository is not configured. Cannot proceed further");
        return Ok(());
    }

    let reference_repo_path = profile.reference_repo.as_ref().unwrap();
    let mut packages_to_copy =
        alpm_helper::get_newer_packages_from_reference(&profile.repo, reference_repo_path)
            .context("Failed to get newer packages from reference repo")?;

    if !packages_to_copy.is_empty() {
        // NOTE: probably we would rather want here to see filenames instead of full paths
        tracing::info!("Found newer packages in ref repo: {packages_to_copy:?}");
    }

    // lets invalidate packages if they are without signatures
    if !pkg_utils::validate_packages(profile.require_signature, &packages_to_copy) {
        tracing::error!("Aborting due to found 'invalid' packages. Cannot proceed further");
        return Ok(());
    }

    let debug_pkgs = if profile.debug_repo.is_some() {
        pkg_utils::exclude_debug_pkgs(&mut packages_to_copy)
    } else {
        vec![]
    };

    // Copy the packages to the profile repository directory
    for package_path in &packages_to_copy {
        let ref_pkg = pkg_utils::get_pkg_db_pair_from_path(package_path);
        tracing::info!("ref repo: {ref_pkg}");

        if let Err(pkg_copy_err) = handle_pkgfile_copy(package_path, repo_dir.to_str().unwrap()) {
            tracing::error!("Error occurred while copying package files: {pkg_copy_err}");
            return Ok(());
        }
    }

    route_debug_pkgs_to_debug_repo(profile, &debug_pkgs, true)?;

    // TODO: handle new packages(which dont exist in repo, but exist in ref repo), handle stale
    // packages(which no longer exist in ref repo)

    // report status only when had some work
    if packages_to_copy.is_empty() {
        tracing::info!("nothing to do");
    } else {
        tracing::info!("Repo ref sync is done!");
    }

    Ok(())
}

async fn do_repo_move_pkgs(
    profile: &config::Profile,
    repo_dir: &Path,
    pg_helper: Option<PostgresqlHelper>,
) -> Result<()> {
    // 1. moving packages from current dir
    let current_dir = std::env::current_dir().context("Failed to get current working dir")?;

    // here we get only packages without signature
    let mut pkg_to_move_list = pkg_utils::find_packages_in_dir(current_dir.as_path())
        .context("Failed to get package files in current working dir")?;

    if !pkg_to_move_list.is_empty() {
        // NOTE: probably we would rather want here to see filenames instead of full paths
        tracing::info!("Found packages to move in current dir: {pkg_to_move_list:?}");
    }

    // lets invalidate packages if they are without signatures
    if !pkg_utils::validate_packages(profile.require_signature, &pkg_to_move_list) {
        tracing::error!("Aborting due to found 'invalid' packages. Cannot proceed further");
        return Ok(());
    }

    // lets invalidate packages if they are already in the target repo (and are not newer versions)
    let already_in_repo = pkg_utils::exclude_existing_pkgs(&profile.repo, &pkg_to_move_list);

    if !already_in_repo.is_empty() {
        tracing::warn!(
            "Found packages already in the repo: {already_in_repo:?}, excluding them from move"
        );
        pkg_to_move_list.retain(|pkg| !already_in_repo.contains(pkg));
    }

    let debug_pkgs = if profile.debug_repo.is_some() {
        pkg_utils::exclude_debug_pkgs(&mut pkg_to_move_list)
    } else {
        vec![]
    };

    if let Err(pkg_move_err) = handle_pkgfiles_move(&pkg_to_move_list, repo_dir.to_str().unwrap()) {
        tracing::error!("Error occurred while moving package files: {pkg_move_err}");
        return Ok(());
    }

    // 2. doing regular repo update
    // TODO(vnepogodin): don't parse all packages in the repo,
    // we need to touch only packages which we move into
    do_repo_update(profile, repo_dir, pg_helper).await?;

    // 2.1. move debug packages into the debug dir if configured
    route_debug_pkgs_to_debug_repo(profile, &debug_pkgs, false)?;

    // report status only when had some work
    if pkg_to_move_list.is_empty() {
        tracing::info!("nothing to do");
    } else {
        tracing::info!("Repo MovePkgsToRepo is done!");
    }

    Ok(())
}

async fn do_repo_checkup(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    let pkgs_list = pkg_utils::find_packages_in_dir(repo_dir)?;

    let outdated_pkgs = pkg_utils::get_outdated_pkgs(&pkgs_list);
    let new_pkgs = pkg_utils::get_new_pkgs(&pkgs_list);

    let repo_db_prefix = pkg_utils::get_repo_db_prefix(&profile.repo);

    // 1. handle new packages

    // handle new packages which are not present in the DB
    let brand_new_pkgs = alpm_helper::get_brand_new_packages(&profile.repo)
        .context("Failed to get brand new pkgs")?;

    for brand_new_pkg in brand_new_pkgs {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&brand_new_pkg);
        tracing::info!("Found brand new package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    for new_pkg in new_pkgs {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&new_pkg);
        tracing::info!("Found new package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    // 1.1 handle removal/backup of old packages here
    for outdated_pkg in outdated_pkgs {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&outdated_pkg);
        tracing::info!("Found outdated package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    // 2. handle stale packages

    // we want to get here filenames of stale packages
    let stale_filenames = alpm_helper::get_stale_filenames(&profile.repo)
        .context("Failed to get stale pkgs with filename")?;

    for stale_filename in stale_filenames {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&stale_filename);
        tracing::info!("Found stale package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    // 3. handle ref repository
    // Check for newer packages in the reference repository
    if let Some(reference_repo_path) = &profile.reference_repo {
        let packages_to_copy =
            alpm_helper::get_newer_packages_from_reference(&profile.repo, reference_repo_path)
                .context("Failed to get newer packages from reference repo")?;

        if !packages_to_copy.is_empty() {
            let new_pkgname_list = packages_to_copy
                .iter()
                .map(|x| pkg_utils::get_pkg_db_pair_from_path(x))
                .collect::<Vec<_>>();
            tracing::info!("Found new pkgs from ref repo '{repo_db_prefix}': {new_pkgname_list:?}");
        }
    }

    // 4. report debug packages that are still in the prod repo
    if let Some(ref debug_repo) = profile.debug_repo
        && debug_repo != &profile.repo
    {
        let debug_pkgs = pkg_utils::get_debug_packages(&pkgs_list);
        for debug_pkg in &debug_pkgs {
            let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(debug_pkg);
            tracing::info!("Found debug package in repo '{repo_db_prefix}': '{pkg_pair}'");
        }
    }

    tracing::info!("Repo checkup is done!");

    Ok(())
}

async fn do_repo_aur(repo_dir: &Path, order_file: Option<PathBuf>, dry_run: bool) -> Result<()> {
    // NOTE: looks ugly, but we only need db pair
    let pkgs_list = pkg_utils::find_packages_in_dir(repo_dir)?;
    let new_pkgs = pkgs_list
        .iter()
        .map(|x| {
            let filename = Path::new(x).file_name().unwrap().to_str().unwrap();
            let pkgname = pkg_utils::get_pkgname_from_filename(filename).to_owned();
            let pkgver = pkg_utils::get_pkgver_from_filename(filename).to_owned();
            (pkgname, pkgver)
        })
        .filter(|x| {
            // filter out -git packages which will be always out-of-date
            !x.0.contains("-git")
        })
        .collect::<Vec<_>>();

    let package_summary =
        aur::get_new_aur_pkgs(&new_pkgs).await.context("Failed to get new AUR pkgs")?;
    for new_pkg in &package_summary.new_pkgs {
        tracing::info!("Found new AUR package: '{}-{}'", new_pkg.name, new_pkg.version);
    }

    if !dry_run {
        let current_dir = env::current_dir()?;
        aur::pull_tarballs(&package_summary.build_order, &current_dir)
            .await
            .context("Failed to pull sources")?;
    }

    // write order if user requested
    if let Some(order_file) = order_file {
        let file_content = package_summary
            .build_order
            .iter()
            .map(|x| x.name.clone())
            .collect::<Vec<_>>()
            .join("\n");
        tokio::fs::write(order_file, file_content)
            .await
            .context("Failed to write build order to file")?;
    }

    Ok(())
}

/// Transfer already-extracted debug package files to the debug repo dir and add to its ALPM DB.
fn route_debug_pkgs_to_debug_repo(
    profile: &config::Profile,
    debug_pkgs: &[String],
    copy: bool,
) -> Result<()> {
    let debug_repo = match &profile.debug_repo {
        Some(dr) if dr != &profile.repo => dr,
        _ => return Ok(()),
    };
    if debug_pkgs.is_empty() {
        return Ok(());
    }

    tracing::debug!("Found debug packages: {debug_pkgs:?}");
    let debug_dir = Path::new(debug_repo).parent().unwrap();
    fs::create_dir_all(debug_dir)?;

    let debug_dir_str = debug_dir.to_str().unwrap();
    if copy {
        handle_pkgfiles_copy(debug_pkgs, debug_dir_str)?;
    } else {
        handle_pkgfiles_move(debug_pkgs, debug_dir_str)?;
    }
    let debug_files = pkg_utils::replace_base_dir_for_pkgs(debug_pkgs, debug_dir);
    repo_utils::repo_add(debug_repo, &profile.add_params, &debug_files)?;

    let action = if copy { "Copied" } else { "Moved" };
    tracing::info!("{action} {} debug package(s) to debug repo", debug_pkgs.len());
    Ok(())
}

// Runs through the backup folder, and removes the backup of versions which we don't want to keep
fn do_backup_repo_cleanup(profile: &config::Profile) -> Result<()> {
    if !profile.backup || profile.backup_dir == Some(profile.repo.clone()) {
        tracing::info!("Backup is disabled for this repo");
        return Ok(());
    }

    if profile.backup_num.is_none() {
        tracing::info!(
            "Backup is enabled, but the versions of backup packages in the repo is unlimited for \
             this repo"
        );
        return Ok(());
    }

    // lets get all packages in the repo it self and the debug repo folder
    let backup_dir = Path::new(profile.backup_dir.as_ref().unwrap());
    let pkgs_list = pkg_utils::find_packages_in_dir(backup_dir)?;

    let mut pkg_map =
        pkg_utils::get_stale_pkg_versions(&pkgs_list, *profile.backup_num.as_ref().unwrap());
    for (name, versions) in &mut pkg_map {
        // Remove the packages with more than N versions
        let pkg_versions = versions.iter().map(|x| x.1.to_string()).collect::<Vec<_>>();
        tracing::info!(
            "Found more backup versions of package({name}) than allowed: {pkg_versions:?}"
        );

        // TODO(vnepogodin): make a prompt on every run here in case iteractive is on
        for filepath in versions.iter().map(|x| &x.0) {
            tracing::debug!("Removing package version: {filepath}");

            // remove the actual package file
            if let Err(file_err) = fs::remove_file(filepath) {
                tracing::error!("Failed to remove the backup file '{filepath}': {file_err}");
            }

            // remove package signature
            let sig_filepath = format!("{filepath}.sig");
            if Path::new(&sig_filepath).exists()
                && let Err(file_err) = fs::remove_file(&sig_filepath)
            {
                tracing::error!(
                    "Failed to remove the backup file sig '{sig_filepath}': {file_err}"
                );
            }
        }
    }

    tracing::info!("The cleanup of backups is done!");

    Ok(())
}

// Transfers packages from one repo to another repo
// 1. moves package files in the src repo to the dest repo
// 2. removes packages from the src repo DB
// 3. adds packages to the dest repo DB
async fn move_packages_from_repo_to_repo(
    src_profile: &Profile,
    src_repo_dir: &Path,
    dest_profile: &Profile,
    dest_repo_dir: &Path,
    pg_helper: Option<PostgresqlHelper>,
) -> Result<()> {
    // here we get only packages without signature
    let pkg_to_move_list = pkg_utils::find_packages_in_dir(src_repo_dir)?;

    // NOTE: probably we would rather want here to see filenames instead of full paths
    tracing::info!("Found packages to move in src dir: {pkg_to_move_list:?}");

    // lets invalidate packages if they are without signatures
    if !pkg_utils::validate_packages(dest_profile.require_signature, &pkg_to_move_list) {
        tracing::error!("Aborting due to found 'invalid' packages. Cannot proceed further");
        return Ok(());
    }

    if let Err(pkg_move_err) =
        handle_pkgfiles_move(&pkg_to_move_list, dest_repo_dir.to_str().unwrap())
    {
        tracing::error!("Error occurred while moving package files: {pkg_move_err}");
        return Ok(());
    }

    // modify source repo DB (e.g remove the moved packages from the db)
    let added_pkgs_files = pkg_utils::replace_base_dir_for_pkgs(&pkg_to_move_list, dest_repo_dir);
    let removal_pkgs =
        alpm_helper::get_packages_from_filepaths(&src_profile.repo, &pkg_to_move_list)?;

    repo_utils::handle_repo_remove(src_profile, &removal_pkgs)?;
    repo_utils::handle_repo_add(dest_profile, &added_pkgs_files)?;

    // Update db if configured
    if let Some(ref pg) = pg_helper {
        let mut tx = pg.begin().await?;
        let srcrepo_name = pkg_utils::get_repo_db_prefix(&src_profile.repo);
        PostgresqlHelper::remove_packages_on(&mut tx, &srcrepo_name, &removal_pkgs).await?;
        alpm_helper::add_pkgs_to_db(&dest_profile.repo, &mut tx, &added_pkgs_files).await?;
        tx.commit().await.context("Failed to commit move packages transaction")?;
    }

    tracing::info!("Repo MovePkgsFromRepo2Repo is done!");

    Ok(())
}

fn do_check_rebuild(repo_dir: &Path, pacman_conf: Option<&Path>) -> Result<()> {
    let broken = rebuild_check::check_broken_packages(repo_dir, pacman_conf)?;
    if broken.is_empty() {
        tracing::info!("All packages in {} have satisfied .so dependencies.", repo_dir.display());
        return Ok(());
    }

    tracing::warn!("Broken packages (need rebuild):");
    for pkg in &broken {
        let libs = pkg.missing_libs.join(", ");
        tracing::warn!("  {} {}: {libs}", pkg.name, pkg.version);
    }
    tracing::error!("Found {} broken package(s).", broken.len());
    anyhow::bail!("{} broken package(s) found", broken.len());
}

fn handle_outdated_pkgs(profile: &config::Profile, outdated_pkgs: &[String]) -> Result<()> {
    // 1. handle removal/backup here
    tracing::debug!("outdated_pkgs := {outdated_pkgs:?}");
    for outdated_pkg in outdated_pkgs {
        let outdated_pkg_entry = pkg_utils::get_pkg_db_pair_from_path(outdated_pkg);

        // TODO(vnepogodin): make a prompt on every run here in case iteractive is on
        if profile.backup && profile.backup_dir != Some(profile.repo.clone()) {
            tracing::info!("backup '{outdated_pkg_entry}'..");
            handle_pkgfile_move(outdated_pkg, profile.backup_dir.as_ref().unwrap())?;
        } else {
            tracing::info!("rm '{outdated_pkg_entry}'..");
            // we would rather be fail safe here and just report without *panicing*
            if let Err(rm_err) = fs::remove_file(outdated_pkg) {
                tracing::error!("Failed to remove outdated package '{outdated_pkg}': {rm_err}");
            }

            // remove package signature
            let sig_filepath = format!("{outdated_pkg}.sig");
            if Path::new(&sig_filepath).exists()
                && let Err(file_err) = fs::remove_file(&sig_filepath)
            {
                tracing::error!(
                    "Failed to remove outdated package sig '{sig_filepath}': {file_err}"
                );
            }
        }
    }

    // 2. handle stale backups here
    // to not spam the log with needless run
    if profile.backup {
        // lets run just regular backup cleanup
        do_backup_repo_cleanup(profile)?;
    }

    // 3. clean up outdated debug package files in the debug repo dir
    if let Some(ref debug_repo) = profile.debug_repo
        && debug_repo != &profile.repo
    {
        let debug_dir = Path::new(debug_repo).parent().unwrap();
        if debug_dir.exists() {
            let debug_pkgs = pkg_utils::find_packages_in_dir(debug_dir)?;
            let outdated_debug = pkg_utils::get_outdated_pkgs(&debug_pkgs);
            for pkg in &outdated_debug {
                let entry = pkg_utils::get_pkg_db_pair_from_path(pkg);
                tracing::info!("rm outdated debug pkg '{entry}'..");
                if let Err(e) = fs::remove_file(pkg) {
                    tracing::error!("Failed to remove outdated debug package '{pkg}': {e}");
                }
                let sig = format!("{pkg}.sig");
                if Path::new(&sig).exists() {
                    let _ = fs::remove_file(&sig);
                }
            }
        }
    }

    Ok(())
}

fn handle_pkgfile_copy(pkg_to_copy: &str, dest_dir: &str) -> Result<()> {
    let pkg_filename = Path::new(&pkg_to_copy).file_name().unwrap().to_str().unwrap();
    let dest_path = format!("{dest_dir}/{pkg_filename}");

    // NOTE: maybe we should change log level depending on the func argument,
    // we may not want to have it all time as info, for example at handling outdated packages
    tracing::info!("Copying pkg from '{pkg_to_copy}' -> '{dest_path}'");

    // copying package
    if let Err(copy_err) = fs::copy(pkg_to_copy, &dest_path) {
        anyhow::bail!("Failed to copy pkg: {copy_err}");
    }
    // copying package signature
    let pkg_sig_to_copy = format!("{pkg_to_copy}.sig");
    let sig_dest_path = format!("{dest_path}.sig");
    if Path::new(&pkg_sig_to_copy).exists()
        && let Err(copy_err) = fs::copy(pkg_sig_to_copy, &sig_dest_path)
    {
        tracing::error!("Failed to copy pkg signature: {copy_err}");
    }

    Ok(())
}

fn handle_pkgfile_move(pkg_to_move: &str, dest_dir: &str) -> Result<()> {
    let pkg_filename = Path::new(&pkg_to_move).file_name().unwrap().to_str().unwrap();
    let dest_path = format!("{dest_dir}/{pkg_filename}");

    // NOTE: maybe we should change log level depending on the func argument,
    // we may not want to have it all time as info, for example at handling outdated packages
    tracing::info!("Moving pkg from '{pkg_to_move}' -> '{dest_path}'");

    // NOTE: maybe we should handle move part better?

    // moving package
    if let Err(move_err) = fs::rename(pkg_to_move, &dest_path) {
        anyhow::bail!("Failed to move pkg: {move_err}");
    }
    // moving package signature
    let pkg_sig_to_move = format!("{pkg_to_move}.sig");
    let sig_dest_path = format!("{dest_path}.sig");
    if Path::new(&pkg_sig_to_move).exists()
        && let Err(move_err) = fs::rename(pkg_sig_to_move, &sig_dest_path)
    {
        tracing::error!("Failed to move pkg signature: {move_err}");
    }

    Ok(())
}

fn handle_pkgfiles_copy(pkg_to_copy_list: &[String], dest_dir: &str) -> Result<()> {
    // now lets copy
    for pkg_to_copy in pkg_to_copy_list {
        handle_pkgfile_copy(pkg_to_copy, dest_dir)?;
    }

    Ok(())
}

fn handle_pkgfiles_move(pkg_to_move_list: &[String], dest_dir: &str) -> Result<()> {
    // now lets move
    for pkg_to_move in pkg_to_move_list {
        handle_pkgfile_move(pkg_to_move, dest_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod debug_pkg_tests {
    use super::*;

    fn create_dummy_pkg(dir: &Path, filename: &str) {
        let pkg_path = dir.join(filename);
        let sig_path = dir.join(format!("{filename}.sig"));
        fs::write(&pkg_path, b"dummy").unwrap();
        fs::write(&sig_path, b"sig").unwrap();
    }

    fn make_profile(debug_repo: Option<String>) -> config::Profile {
        config::Profile {
            repo: "/tmp/fake/repo.db.tar.zst".to_string(),
            add_params: vec![],
            rm_params: vec![],
            require_signature: false,
            backup: false,
            backup_dir: None,
            backup_num: None,
            debug_repo,
            interactive: false,
            reference_repo: None,
        }
    }

    #[test]
    fn test_debug_pkgs_stripped_and_moved() {
        let prod = tempfile::tempdir().unwrap();
        let debug = tempfile::tempdir().unwrap();

        create_dummy_pkg(prod.path(), "foo-1.0-1-x86_64.pkg.tar.zst");
        create_dummy_pkg(prod.path(), "foo-debug-1.0-1-x86_64.pkg.tar.zst");

        let mut pkgs = pkg_utils::find_packages_in_dir(prod.path()).unwrap();
        let debug_pkgs = pkg_utils::exclude_debug_pkgs(&mut pkgs);

        assert_eq!(pkgs.len(), 1);
        assert!(pkgs[0].contains("foo-1.0"));
        assert_eq!(debug_pkgs.len(), 1);
        assert!(debug_pkgs[0].contains("foo-debug"));

        handle_pkgfiles_move(&debug_pkgs, debug.path().to_str().unwrap()).unwrap();

        assert!(debug.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
        assert!(debug.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst.sig").exists());
        assert!(prod.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
        assert!(prod.path().join("foo-1.0-1-x86_64.pkg.tar.zst.sig").exists());
        assert!(!prod.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
    }

    #[test]
    fn test_all_debug_pkgs_moved_from_prod() {
        let prod = tempfile::tempdir().unwrap();
        let debug = tempfile::tempdir().unwrap();

        create_dummy_pkg(prod.path(), "bar-debug-2.0-1-x86_64.pkg.tar.zst");
        create_dummy_pkg(prod.path(), "baz-debug-1.0-1-x86_64.pkg.tar.zst");

        let mut pkgs = pkg_utils::find_packages_in_dir(prod.path()).unwrap();
        let debug_pkgs = pkg_utils::exclude_debug_pkgs(&mut pkgs);

        assert!(pkgs.is_empty());
        assert_eq!(debug_pkgs.len(), 2);

        handle_pkgfiles_move(&debug_pkgs, debug.path().to_str().unwrap()).unwrap();

        assert!(debug.path().join("bar-debug-2.0-1-x86_64.pkg.tar.zst").exists());
        assert!(debug.path().join("baz-debug-1.0-1-x86_64.pkg.tar.zst").exists());

        let remaining = pkg_utils::find_packages_in_dir(prod.path()).unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_non_debug_pkgs_untouched() {
        let prod = tempfile::tempdir().unwrap();

        create_dummy_pkg(prod.path(), "foo-1.0-1-x86_64.pkg.tar.zst");
        create_dummy_pkg(prod.path(), "bar-2.0-1-x86_64.pkg.tar.zst");

        let mut pkgs = pkg_utils::find_packages_in_dir(prod.path()).unwrap();
        let debug_pkgs = pkg_utils::exclude_debug_pkgs(&mut pkgs);

        assert!(debug_pkgs.is_empty());
        assert_eq!(pkgs.len(), 2);

        assert!(prod.path().join("foo-1.0-1-x86_64.pkg.tar.zst").exists());
        assert!(prod.path().join("bar-2.0-1-x86_64.pkg.tar.zst").exists());
    }

    #[test]
    fn test_no_debug_repo_configured_is_noop() {
        let profile = make_profile(None);
        let prod = tempfile::tempdir().unwrap();
        create_dummy_pkg(prod.path(), "foo-debug-1.0-1-x86_64.pkg.tar.zst");

        let mut pkgs = pkg_utils::find_packages_in_dir(prod.path()).unwrap();
        let debug_pkgs = pkg_utils::exclude_debug_pkgs(&mut pkgs);
        let result = route_debug_pkgs_to_debug_repo(&profile, &debug_pkgs, false);
        assert!(result.is_ok());

        assert!(prod.path().join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
    }

    #[test]
    fn test_debug_pkgs_moved_to_new_dir() {
        let prod = tempfile::tempdir().unwrap();
        let base = tempfile::tempdir().unwrap();
        let debug_dir = base.path().join("new_debug_dir");

        create_dummy_pkg(prod.path(), "foo-debug-1.0-1-x86_64.pkg.tar.zst");

        let mut pkgs = pkg_utils::find_packages_in_dir(prod.path()).unwrap();
        let debug_pkgs = pkg_utils::exclude_debug_pkgs(&mut pkgs);

        assert!(!debug_dir.exists());
        fs::create_dir_all(&debug_dir).unwrap();
        handle_pkgfiles_move(&debug_pkgs, debug_dir.to_str().unwrap()).unwrap();

        assert!(debug_dir.join("foo-debug-1.0-1-x86_64.pkg.tar.zst").exists());
    }
}
