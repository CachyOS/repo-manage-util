mod alpm_helper;
mod args;
mod config;
mod logger;
mod pg_types;
mod pkg_utils;
mod postgresql_helper;
mod repo_utils;
mod utils;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use args::*;
use clap::Parser;
use config::Profile;
use postgresql_helper::PostgresqlHelper;

fn get_profile_from_config<'a>(
    profile_name: &'a str,
    config: &'a config::Config,
) -> Result<&'a config::Profile> {
    config.profiles.get(profile_name).ok_or(anyhow::anyhow!("Profile {} not found", profile_name))
}

fn get_repo_dir_from_profile(profile: &config::Profile) -> &Path {
    Path::new(&profile.repo).parent().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // initialize the logger
    logger::init_logger().expect("Failed to initialize logger");

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

            log::debug!("repo db path := {repo_db_pattern}");

            do_repo_reset(profile, &repo_db_pattern, repo_dir, pg_helper, args.only_pg).await?;
            // TODO(vnepogodin): handle debug packages
            // move them to debug folder if is set
        },
        Commands::Update(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_update(profile, repo_dir, pg_helper).await?;
            // TODO(vnepogodin): handle debug packages
            // move them to debug folder if is set
        },
        Commands::Sync(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_sync(profile, repo_dir).await?;
            // TODO(vnepogodin): handle debug packages
            // move them to debug folder if is set
        },
        Commands::MovePkgsToRepo(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_move_pkgs(profile, repo_dir, pg_helper).await?;
            // TODO(vnepogodin): handle debug packages
            // move them to debug folder if is set
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
            log::debug!("removing db file '{pattern}'..");
            for entry in glob::glob(pattern)? {
                fs::remove_file(entry?)?
            }
        }

        let mut pkgs_list = pkg_utils::find_packages_in_dir(repo_dir)?;
        let outdated_pkgs = pkg_utils::get_outdated_pkgs(&pkgs_list);
        pkgs_list.retain(|pkg| !outdated_pkgs.contains(pkg));

        // don't insert packages without signature
        if profile.require_signature {
            pkg_utils::remove_pkgs_without_sig(&mut pkgs_list);
        }

        // run repo-add
        repo_utils::handle_repo_add(profile, &pkgs_list)?;

        // handle removal/backup here
        handle_outdated_pkgs(profile, &outdated_pkgs)?;
    }

    // Reset db if configured
    if let Some(ref pg) = pg_helper {
        // purge repo packages first then populate
        let repo_name = pkg_utils::get_repo_db_prefix(&profile.repo);
        pg.remove_existing_repository(&repo_name).await?;
        alpm_helper::populate_repo_to_db(&profile.repo, pg).await?;
    }

    log::info!("Repo reset is done!");

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

    // 2. handle stale packages
    let stale_pkgs =
        alpm_helper::get_stale_packages(&profile.repo).context("Failed to get stale pkgs")?;

    // if we found stale packages then remove them from DB
    // overwise silently skip and finish update command
    if !stale_pkgs.is_empty() {
        repo_utils::handle_repo_remove(profile, &stale_pkgs)?;
    }

    // Update db if configured
    if let Some(ref pg) = pg_helper {
        let repo_name = pkg_utils::get_repo_db_prefix(&profile.repo);
        pg.remove_packages(&repo_name, &stale_pkgs).await?;
        alpm_helper::add_pkgs_to_db(&profile.repo, pg, &new_pkgs).await?;
    }

    // report status only when had some work
    if !new_pkgs.is_empty() || !stale_pkgs.is_empty() {
        log::info!("Repo update is done!");
    } else {
        log::info!("nothing to do");
    }

    Ok(())
}

async fn do_repo_sync(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    if profile.reference_repo.is_none() {
        log::error!("Reference repository is not configured. Cannot proceed further");
        return Ok(());
    }

    let reference_repo_path = profile.reference_repo.as_ref().unwrap();
    let packages_to_copy =
        alpm_helper::get_newer_packages_from_reference(&profile.repo, reference_repo_path)
            .context("Failed to get newer packages from reference repo")?;

    if !packages_to_copy.is_empty() {
        // NOTE: probably we would rather want here to see filenames instead of full paths
        log::info!("Found newer packages in ref repo: {packages_to_copy:?}");
    }

    // lets invalidate packages if they are without signatures
    if !pkg_utils::validate_packages(profile.require_signature, &packages_to_copy) {
        log::error!("Aborting due to found 'invalid' packages. Cannot proceed further");
        return Ok(());
    }

    // Copy the packages to the profile repository directory
    for package_path in &packages_to_copy {
        let ref_pkg = pkg_utils::get_pkg_db_pair_from_path(package_path);
        log::info!("ref repo: {ref_pkg}");

        if let Err(pkg_copy_err) = handle_pkgfile_copy(package_path, repo_dir.to_str().unwrap()) {
            log::error!("Error occurred while copying package files: {pkg_copy_err}");
            return Ok(());
        }
    }

    // TODO: handle new packages(which dont exist in repo, but exist in ref repo), handle stale
    // packages(which no longer exist in ref repo)

    // report status only when had some work
    if !packages_to_copy.is_empty() {
        log::info!("Repo ref sync is done!");
    } else {
        log::info!("nothing to do");
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
        log::info!("Found packages to move in current dir: {pkg_to_move_list:?}");
    }

    // lets invalidate packages if they are without signatures
    if !pkg_utils::validate_packages(profile.require_signature, &pkg_to_move_list) {
        log::error!("Aborting due to found 'invalid' packages. Cannot proceed further");
        return Ok(());
    }

    // lets invalidate packages if they are already in the target repo (and are not newer versions)
    let already_in_repo = pkg_utils::exclude_existing_pkgs(&profile.repo, &pkg_to_move_list);

    if !already_in_repo.is_empty() {
        log::warn!(
            "Found packages already in the repo: {already_in_repo:?}, excluding them from move"
        );
        pkg_to_move_list.retain(|pkg| !already_in_repo.contains(pkg));
    }

    if let Err(pkg_move_err) = handle_pkgfiles_move(&pkg_to_move_list, repo_dir.to_str().unwrap()) {
        log::error!("Error occurred while moving package files: {pkg_move_err}");
        return Ok(());
    }

    // 2. doing regular repo update
    // TODO(vnepogodin): don't parse all packages in the repo,
    // we need to touch only packages which we move into
    do_repo_update(profile, repo_dir, pg_helper).await?;

    // report status only when had some work
    if !pkg_to_move_list.is_empty() {
        log::info!("Repo MovePkgsToRepo is done!");
    } else {
        log::info!("nothing to do");
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
        log::info!("Found brand new package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    for new_pkg in new_pkgs {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&new_pkg);
        log::info!("Found new package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    // 1.1 handle removal/backup of old packages here
    for outdated_pkg in outdated_pkgs {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&outdated_pkg);
        log::info!("Found outdated package in repo '{repo_db_prefix}': '{pkg_pair}'");
    }

    // 2. handle stale packages

    // we want to get here filenames of stale packages
    let stale_filenames = alpm_helper::get_stale_filenames(&profile.repo)
        .context("Failed to get stale pkgs with filename")?;

    for stale_filename in stale_filenames {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(&stale_filename);
        log::info!("Found stale package in repo '{repo_db_prefix}': '{pkg_pair}'");
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
            log::info!("Found new pkgs from ref repo '{repo_db_prefix}': {new_pkgname_list:?}");
        }
    }

    log::info!("Repo checkup is done!");

    Ok(())
}

fn do_debug_packages_check(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    // 1. check if we have debug repo assigned
    if profile.debug_dir.is_none() || profile.debug_dir == Some(profile.repo.clone()) {
        log::info!("Separate debug repo is disabled for this profile");
        return Ok(());
    }

    // NOTE: lets just move debug packages into the directory of the repo
    // don't touch the debug repo DB at all.

    // 2. get all debug packages in the repo it self, to move them into the debug directory
    let pkgs_list = glob::glob(&format!("{}/*-debug-*.pkg.tar.zst", repo_dir.to_str().unwrap()))?
        .map(|x| x.unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    // // the debug_dir is the parent dir without the repo
    // if let Some(debug_dir) = &profile.debug_dir {
    //     let debug_pkgs_list =
    //         glob::glob(&format!("{}/*-debug-*.pkg.tar.zst",
    // profile.debug_dir.as_ref().unwrap()))?             .map(|x|
    // x.unwrap().to_str().unwrap().to_owned())             .collect::<Vec<_>>();

    //     pkgs_list.append(&mut debug_pkgs_list);
    // }

    // TODO(vnepogodin): make a prompt on every run here in case iteractive is on
    for pkg_to_move in &pkgs_list
    // .iter().map(|x| Path::new(x))
    {
        let pkg_pair = pkg_utils::get_pkg_db_pair_from_path(pkg_to_move);
        log::debug!("Found debug package in repo: {pkg_pair}");
        // log::debug!("Moving debug package into debug dir: {pkg_to_move}");
        // if let Err(file_err) = fs::rename_file(filepath) {
        //     log::error!("Failed to move the debug package '{filepath}': {file_err}");
        // }
    }

    Ok(())
}

// Runs through the backup folder, and removes the backup of versions which we don't want to keep
fn do_backup_repo_cleanup(profile: &config::Profile) -> Result<()> {
    if !profile.backup || profile.backup_dir == Some(profile.repo.clone()) {
        log::info!("Backup is disabled for this repo");
        return Ok(());
    }

    if profile.backup_num.is_none() {
        log::info!(
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
    for (name, versions) in pkg_map.iter_mut() {
        // Remove the packages with more than N versions
        let pkg_versions = versions.iter().map(|x| x.1.to_string()).collect::<Vec<_>>();
        log::info!("Found more backup versions of package({name}) than allowed: {pkg_versions:?}");

        // TODO(vnepogodin): make a prompt on every run here in case iteractive is on
        for filepath in versions.iter().map(|x| &x.0) {
            log::debug!("Removing package version: {filepath}");

            // remove the actual package file
            if let Err(file_err) = fs::remove_file(filepath) {
                log::error!("Failed to remove the backup file '{filepath}': {file_err}");
            }

            // remove package signature
            let sig_filepath = format!("{filepath}.sig");
            if Path::new(&sig_filepath).exists()
                && let Err(file_err) = fs::remove_file(&sig_filepath)
            {
                log::error!("Failed to remove the backup file sig '{sig_filepath}': {file_err}");
            }
        }
    }

    log::info!("The cleanup of backups is done!");

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
    log::info!("Found packages to move in src dir: {pkg_to_move_list:?}");

    // lets invalidate packages if they are without signatures
    if !pkg_utils::validate_packages(dest_profile.require_signature, &pkg_to_move_list) {
        log::error!("Aborting due to found 'invalid' packages. Cannot proceed further");
        return Ok(());
    }

    if let Err(pkg_move_err) =
        handle_pkgfiles_move(&pkg_to_move_list, dest_repo_dir.to_str().unwrap())
    {
        log::error!("Error occurred while moving package files: {pkg_move_err}");
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
        let srcrepo_name = pkg_utils::get_repo_db_prefix(&src_profile.repo);
        pg.remove_packages(&srcrepo_name, &removal_pkgs).await?;
        alpm_helper::add_pkgs_to_db(&dest_profile.repo, pg, &added_pkgs_files).await?;
    }

    log::info!("Repo MovePkgsFromRepo2Repo is done!");

    Ok(())
}

fn handle_outdated_pkgs(profile: &config::Profile, outdated_pkgs: &[String]) -> Result<()> {
    // 1. handle removal/backup here
    log::debug!("outdated_pkgs := {outdated_pkgs:?}");
    for outdated_pkg in outdated_pkgs {
        let outdated_pkg_entry = pkg_utils::get_pkg_db_pair_from_path(outdated_pkg);

        // TODO(vnepogodin): make a prompt on every run here in case iteractive is on
        if profile.backup && profile.backup_dir != Some(profile.repo.clone()) {
            log::info!("backup '{outdated_pkg_entry}'..");
            handle_pkgfile_move(outdated_pkg, profile.backup_dir.as_ref().unwrap())?;
        } else {
            log::info!("rm '{outdated_pkg_entry}'..");
            // we would rather be fail safe here and just report without *panicing*
            if let Err(rm_err) = fs::remove_file(outdated_pkg) {
                log::error!("Failed to remove outdated package '{outdated_pkg}': {rm_err}");
            }

            // remove package signature
            let sig_filepath = format!("{outdated_pkg}.sig");
            if Path::new(&sig_filepath).exists()
                && let Err(file_err) = fs::remove_file(&sig_filepath)
            {
                log::error!("Failed to remove outdated package sig '{sig_filepath}': {file_err}");
            }
        }
    }

    // 2. handle stale backups here
    // to not spam the log with needless run
    if profile.backup {
        // lets run just regular backup cleanup
        do_backup_repo_cleanup(profile)?;
    }

    Ok(())
}

fn handle_pkgfile_copy(pkg_to_copy: &str, dest_dir: &str) -> Result<()> {
    let pkg_filename = Path::new(&pkg_to_copy).file_name().unwrap().to_str().unwrap();
    let dest_path = format!("{dest_dir}/{pkg_filename}");

    // NOTE: maybe we should change log level depending on the func argument,
    // we may not want to have it all time as info, for example at handling outdated packages
    log::info!("Copying pkg from '{pkg_to_copy}' -> '{dest_path}'");

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
        log::error!("Failed to copy pkg signature: {copy_err}");
    }

    Ok(())
}

fn handle_pkgfile_move(pkg_to_move: &str, dest_dir: &str) -> Result<()> {
    let pkg_filename = Path::new(&pkg_to_move).file_name().unwrap().to_str().unwrap();
    let dest_path = format!("{dest_dir}/{pkg_filename}");

    // NOTE: maybe we should change log level depending on the func argument,
    // we may not want to have it all time as info, for example at handling outdated packages
    log::info!("Moving pkg from '{pkg_to_move}' -> '{dest_path}'");

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
        log::error!("Failed to move pkg signature: {move_err}");
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
