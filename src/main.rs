mod alpm_helper;
mod config;
mod logger;
mod pkg_utils;
mod repo_utils;
mod utils;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use config::Profile;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct SingleProfileCli {
    /// Profile to use from the configuration file
    #[arg(short, long)]
    profile: String,
}

#[derive(Args, Debug)]
struct FromToProfileCli {
    /// Profile to use from the configuration file (for move-pkgs) FROM repo
    #[arg(short, long)]
    from: String,
    /// Profile to use from the configuration file (for move-pkgs) TO repo
    #[arg(short, long)]
    to: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Reset the repository
    Reset(SingleProfileCli),
    /// Update the repository
    Update(SingleProfileCli),
    /// Moves packages from current directory into the repository
    MovePkgsToRepo(SingleProfileCli),
    /// Moves packages from one repository to another repository
    MovePkgs(FromToProfileCli),
    /// Check if the packages are up-to-date
    IsPkgsUpToDate(SingleProfileCli),
    /// Cleans up the backup directory,
    /// removing the N amount of packages if configured to do so
    CleanupBackupDir(SingleProfileCli),
    // Check if we have only certain amount of debug packages in the debug repository
    // IsDebugPkgsOk, // ok maybe not implemented
}

fn get_profile_from_config<'a>(
    profile_name: &'a str,
    config: &'a config::Config,
) -> Result<&'a config::Profile> {
    config.profiles.get(profile_name).ok_or(anyhow::anyhow!("Profile {} not found", profile_name))
}

fn get_repo_dir_from_profile(profile: &config::Profile) -> &Path {
    Path::new(&profile.repo).parent().unwrap()
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // initialize the logger
    logger::init_logger().expect("Failed to initialize logger");

    // load config
    let config_path = config::get_config_path()?;
    let config = config::parse_config_file(&config_path)?;

    match &args.command {
        Commands::Reset(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            let repo_db_prefix = pkg_utils::get_repo_db_prefix(&profile.repo);
            let repo_db_pattern = format!("{}/{repo_db_prefix}.*", repo_dir.to_str().unwrap());

            log::debug!("repo db path := {repo_db_pattern}");

            do_repo_reset(profile, &repo_db_pattern, repo_dir)?;
            // TODO(vnepogodin): handle debug packages
            // move them to debug folder if is set
        },
        Commands::Update(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_update(profile, repo_dir)?;
            // TODO(vnepogodin): handle debug packages
            // move them to debug folder if is set
        },
        Commands::MovePkgsToRepo(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_move_pkgs(profile, repo_dir)?;
        },
        Commands::IsPkgsUpToDate(args) => {
            let profile = get_profile_from_config(&args.profile, &config)?;
            let repo_dir = get_repo_dir_from_profile(profile);

            do_repo_checkup(profile, repo_dir)?;
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

            move_packages_from_repo_to_repo(from_profile, from_repo_dir, to_profile, to_repo_dir)?;
        },
    }

    Ok(())
}

fn do_repo_reset(profile: &config::Profile, repo_db_pattern: &str, repo_dir: &Path) -> Result<()> {
    // Remove db and files
    for pattern in [repo_db_pattern] {
        log::debug!("removing db file '{pattern}'..");
        for entry in glob::glob(pattern)? {
            fs::remove_file(entry?)?
        }
    }

    let mut pkgs_list = glob::glob(&format!("{}/*.pkg.tar.zst", repo_dir.to_str().unwrap()))?
        .map(|x| x.unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();
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

    log::info!("Repo reset is done!");

    Ok(())
}

fn do_repo_update(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    let pkgs_list = glob::glob(&format!("{}/*.pkg.tar.zst", repo_dir.to_str().unwrap()))?
        .map(|x| x.unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let outdated_pkgs = pkg_utils::get_outdated_pkgs(&pkgs_list);
    let mut new_pkgs = pkg_utils::get_new_pkgs(&pkgs_list);

    // 1. handle new packages

    // TODO(vnepogodin): handle ref repo updates here

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

    log::info!("Repo update is done!");

    Ok(())
}

fn do_repo_move_pkgs(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    // 1. moving packages from current dir
    let current_dir = std::env::current_dir().context("Failed to get current working dir")?;

    // here we get only packages without signature
    let mut pkg_to_move_list =
        glob::glob(&format!("{}/*.pkg.tar.zst", current_dir.to_str().unwrap()))?
            .map(|x| x.unwrap().to_str().unwrap().to_owned())
            .collect::<Vec<_>>();

    // NOTE: probably we would rather want here to see filenames instead of full paths
    log::info!("Found packages to move in current dir: {pkg_to_move_list:?}");

    // lets invalidate packages if they are without signatures
    let mut invalid_pkgs: Vec<String> = vec![];
    for pkg_to_move in &pkg_to_move_list {
        // check for signature if we require it
        if profile.require_signature && !Path::new(&format!("{pkg_to_move}.sig")).exists() {
            let pkg_db_entry = pkg_utils::get_pkg_db_pair_from_path(pkg_to_move);
            log::error!("Found package without required signature: '{pkg_db_entry}'");
            invalid_pkgs.push(pkg_to_move.clone());
        }
    }
    if !invalid_pkgs.is_empty() {
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
    do_repo_update(profile, repo_dir)?;

    log::info!("Repo MovePkgsToRepo is done!");

    Ok(())
}

fn do_repo_checkup(profile: &config::Profile, repo_dir: &Path) -> Result<()> {
    let pkgs_list = glob::glob(&format!("{}/*.pkg.tar.zst", repo_dir.to_str().unwrap()))?
        .map(|x| x.unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();
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

        /*
        // Copy the packages to the profile repository directory
        let repo_dir = Path::new(&profile.repo).parent().unwrap();
        for package_path in packages_to_copy {
            let package_filename =
                Path::new(&package_path).file_name().unwrap().to_str().unwrap();
            let destination_path = repo_dir.join(package_filename);

            log::info!("Copying package from reference repository: {}", package_filename);
            fs::copy(&package_path, &destination_path)?;

            // Copy the signature file as well
            let signature_path = format!("{}.sig", package_path);
            if Path::new(&signature_path).exists() {
                let destination_signature_path =
                    format!("{}.sig", destination_path.to_str().unwrap());
                fs::copy(&signature_path, destination_signature_path)?;
            }
        }*/
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
    let pkgs_list = glob::glob(&format!("{}/*.pkg.tar.zst", profile.backup_dir.as_ref().unwrap()))?
        .map(|x| x.unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();

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
            if Path::new(&sig_filepath).exists() {
                if let Err(file_err) = fs::remove_file(&sig_filepath) {
                    log::error!(
                        "Failed to remove the backup file sig '{sig_filepath}': {file_err}"
                    );
                }
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
fn move_packages_from_repo_to_repo(
    src_profile: &Profile,
    src_repo_dir: &Path,
    dest_profile: &Profile,
    dest_repo_dir: &Path,
) -> Result<()> {
    // here we get only packages without signature
    let pkg_to_move_list =
        glob::glob(&format!("{}/*.pkg.tar.zst", src_repo_dir.to_str().unwrap()))?
            .map(|x| x.unwrap().to_str().unwrap().to_owned())
            .collect::<Vec<_>>();

    // NOTE: probably we would rather want here to see filenames instead of full paths
    log::info!("Found packages to move in src dir: {pkg_to_move_list:?}");

    // lets invalidate packages if they are without signatures
    let mut invalid_pkgs: Vec<String> = vec![];
    for pkg_to_move in &pkg_to_move_list {
        // check for signature if we require it
        if dest_profile.require_signature && !Path::new(&format!("{pkg_to_move}.sig")).exists() {
            let pkg_db_entry = pkg_utils::get_pkg_db_pair_from_path(pkg_to_move);
            log::error!("Found package without required signature: '{pkg_db_entry}'");
            invalid_pkgs.push(pkg_to_move.clone());
        }
    }
    if !invalid_pkgs.is_empty() {
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
            if Path::new(&sig_filepath).exists() {
                if let Err(file_err) = fs::remove_file(&sig_filepath) {
                    log::error!(
                        "Failed to remove outdated package sig '{sig_filepath}': {file_err}"
                    );
                }
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

fn handle_pkgfile_move(pkg_to_move: &str, dest_dir: &str) -> Result<()> {
    let pkg_filename = Path::new(&pkg_to_move).file_name().unwrap().to_str().unwrap();
    let dest_path = format!("{}/{pkg_filename}", dest_dir);

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
    if Path::new(&pkg_sig_to_move).exists() {
        if let Err(move_err) = fs::rename(pkg_sig_to_move, &sig_dest_path) {
            log::error!("Failed to move pkg signature: {move_err}");
        }
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
