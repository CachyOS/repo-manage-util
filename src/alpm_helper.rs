use crate::{pkg_utils, utils};

use std::path::Path;
use std::{env, fs};

use alpm::Alpm;
use anyhow::{Context, Result};

#[derive(Debug, PartialEq)]
struct RepoData {
    repo_name: String,
    repo_server: String,
}

fn init_alpm(pacman_path: &str, repo_list: &[RepoData]) -> Result<Alpm> {
    let pacman_db_path = format!("{pacman_path}/db");
    // make sure that it exists, create overwise
    fs::create_dir_all(&pacman_db_path)?;

    let mut handle = Alpm::new(pacman_path, &pacman_db_path)?;

    // add our repos
    for repo_data in repo_list {
        let tmprepo = handle
            .register_syncdb_mut(repo_data.repo_name.clone(), alpm::SigLevel::USE_DEFAULT)
            .unwrap();

        tmprepo.add_server(repo_data.repo_server.clone()).unwrap();
    }

    // update the databases
    handle.syncdbs_mut().update(false)?;

    Ok(handle)
}

fn init_profile_repo(repo_filepath: &str) -> Result<Alpm> {
    let temp_dir = utils::create_temporary_directory(None).expect("Failed to create temp dir");

    let repo_dir =
        Path::new(repo_filepath).parent().expect("Failed to get parent dir from repo filepath");
    let repo_url = format!("file://{}", repo_dir.to_str().unwrap());

    let repo_db_prefix = pkg_utils::get_repo_db_prefix(repo_filepath);

    let repo_list = vec![RepoData { repo_name: repo_db_prefix, repo_server: repo_url }];
    init_alpm(&temp_dir, &repo_list)
}

pub fn exclude_existing_pkgs(repo_db_path: &str, pkgs: &mut pkg_utils::PackageMap) -> Result<Vec<String>> {
    // we iterate through DB with alpm crate, and check for each package
    // if the package exist in the repo and is newer or equal to the one
    // in the list, then we remove it from the list of packages
    let alpm_handle =
        init_profile_repo(repo_db_path).context("Failed to init alpm for src profile repo")?;

    // iterate through every package in the database using map iter
    let mut removed_pkgs: Vec<String> = vec![];
    alpm_handle.syncdbs().iter().flat_map(alpm::Db::pkgs).for_each(|x| {
        if pkgs.contains_key(x.name()) {
            for pkg in pkgs.get(x.name()).unwrap().to_owned() {
                if pkg.1.as_ver() <= x.version() {
                    pkgs.remove(x.name());
                    removed_pkgs.push(pkg.0);
                }
            }
        }
    });

    // cleanup temp dir after we are done
    cleanup_alpm_tempdir(&alpm_handle)?;

    Ok(removed_pkgs)
}

// Gets names of packages from the repo DB using provided filepaths
pub fn get_packages_from_filepaths(
    repo_db_path: &str,
    filepaths: &[String],
) -> Result<Vec<String>> {
    // we iterate through DB with alpm crate, and check for each package
    // if the package file exist in the repo directory and is in the
    // list of filepaths
    let alpm_handle =
        init_profile_repo(repo_db_path).context("Failed to init alpm for stale packages")?;

    let repo_dir = Path::new(&repo_db_path).parent().unwrap();

    // iterate through every package in the database using map iter
    let found_pkgs: Vec<String> = alpm_handle
        .syncdbs()
        .iter()
        .flat_map(alpm::Db::pkgs)
        .filter(|x| {
            // just check if those package exist in the repo and in the list of filepaths
            // if not insert into found pkgs which contains package names
            let pkg_filename = x.filename().expect("Invalid package doesn't have filename");
            let pkg_filepath = format!("{}/{pkg_filename}", repo_dir.to_str().unwrap());
            filepaths.contains(&pkg_filepath)
        })
        .map(|x| x.name().to_string())
        .collect();

    // cleanup temp dir after we are done
    cleanup_alpm_tempdir(&alpm_handle)?;

    Ok(found_pkgs)
}

// Gets names of stale packages from the repo DB
pub fn get_stale_packages(repo_db_path: &str) -> Result<Vec<String>> {
    // we iterate through DB with alpm crate, and check for each package
    // if the package file still exist in the repo directory
    let alpm_handle =
        init_profile_repo(repo_db_path).context("Failed to init alpm for stale packages")?;

    let repo_dir = Path::new(&repo_db_path).parent().unwrap();

    // iterate through every package in the database using map iter
    let stale_pkgs: Vec<String> = alpm_handle
        .syncdbs()
        .iter()
        .flat_map(alpm::Db::pkgs)
        .filter(|x| {
            // just check if those package exist, if not insert into state pkgs which contains
            // package names
            let pkg_filename = x.filename().expect("Invalid package doesn't have filename");
            let pkg_filepath = format!("{}/{pkg_filename}", repo_dir.to_str().unwrap());
            !Path::new(&pkg_filepath).exists()
        })
        .map(|x| x.name().to_string())
        .collect();

    // cleanup temp dir after we are done
    cleanup_alpm_tempdir(&alpm_handle)?;

    Ok(stale_pkgs)
}

// Gets filenames of stale packages from the repo DB
pub fn get_stale_filenames(repo_db_path: &str) -> Result<Vec<String>> {
    // we iterate through DB with alpm crate, and check for each package
    // if the package file still exist in the repo directory
    let alpm_handle =
        init_profile_repo(repo_db_path).context("Failed to init alpm for stale packages")?;

    let repo_dir = Path::new(&repo_db_path).parent().unwrap();

    // iterate through every package in the database using map iter
    let stale_pkgs: Vec<String> = alpm_handle
        .syncdbs()
        .iter()
        .flat_map(alpm::Db::pkgs)
        .filter(|x| {
            // just check if those package exist, if not insert into state pkgs which contains
            // package names
            let pkg_filename = x.filename().expect("Invalid package doesn't have filename");
            let pkg_filepath = format!("{}/{pkg_filename}", repo_dir.to_str().unwrap());
            !Path::new(&pkg_filepath).exists()
        })
        .map(|x| x.filename().unwrap().to_string())
        .collect();

    // cleanup temp dir after we are done
    cleanup_alpm_tempdir(&alpm_handle)?;

    Ok(stale_pkgs)
}

// gets packages which are not yet present in the DB
pub fn get_brand_new_packages(repo_db_path: &str) -> Result<Vec<String>> {
    let repo_dir = Path::new(&repo_db_path).parent().unwrap();

    // get all local packages
    let pkgs_list = glob::glob(&format!("{}/*.pkg.tar.zst", repo_dir.to_str().unwrap()))?
        .map(|x| x.unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    // we iterate through DB with alpm crate, and check for each package in the list
    // if it doesn't, then we found "brand new" package (which doesn't exist yet in DB)
    let alpm_handle =
        init_profile_repo(repo_db_path).context("Failed to init alpm for stale packages")?;

    // iterate through all files and check if they exist in the repo
    let mut new_pkgs: Vec<String> = vec![];
    for pkg_filepath in pkgs_list {
        let pkg_filename = Path::new(&pkg_filepath).file_name().unwrap().to_str().unwrap();
        let pkg_name = crate::pkg_utils::get_pkgname_from_filename(pkg_filename);

        // iterate through each database
        for db in alpm_handle.syncdbs() {
            // if package was not found, then we assume its the new package
            if db.pkg(pkg_name).is_err() {
                new_pkgs.push(pkg_filepath.clone());
            }
        }
    }

    // cleanup temp dir after we are done
    cleanup_alpm_tempdir(&alpm_handle)?;

    Ok(new_pkgs)
}

// Checks the reference repository for newer package versions and returns a list of package
// filepaths to copy.
pub fn get_newer_packages_from_reference(
    repo_db_path: &str,
    reference_repo_path: &str,
) -> Result<Vec<String>> {
    let alpm_handle =
        init_profile_repo(repo_db_path).context("Failed to init alpm for profile repo")?;
    let reference_alpm_handle =
        init_profile_repo(reference_repo_path).context("Failed to init alpm for reference repo")?;

    let mut packages_to_copy = Vec::new();

    // Iterate over packages in the profile repository
    for db in alpm_handle.syncdbs() {
        for pkg in db.pkgs() {
            // Check if the package exists in the reference repository
            if let Some(reference_pkg) = reference_alpm_handle
                .syncdbs()
                .iter()
                .find_map(|ref_db| ref_db.pkg(pkg.name()).ok())
            {
                // Compare versions
                if reference_pkg.version() > pkg.version() {
                    // Newer version found in reference repository
                    let pkg_filename =
                        reference_pkg.filename().expect("Invalid package doesn't have filename");
                    let reference_repo_dir = Path::new(reference_repo_path).parent().unwrap();
                    let pkgfile_path =
                        format!("{}/{pkg_filename}", reference_repo_dir.to_str().unwrap());

                    // skip if the package file doesn't exist in the reference repo
                    if !Path::new(&pkgfile_path).exists() {
                        log::error!("Package file doesn't in ref repo: {pkgfile_path}");
                        continue;
                    }
                    packages_to_copy.push(pkgfile_path);
                }
            }
        }
    }

    // Cleanup temp dirs after we are done
    cleanup_alpm_tempdir(&alpm_handle)?;
    cleanup_alpm_tempdir(&reference_alpm_handle)?;

    Ok(packages_to_copy)
}

fn cleanup_alpm_tempdir(alpm_handle: &Alpm) -> Result<()> {
    let tmp_dir = env::temp_dir();

    let alpm_root_dir = alpm_handle.root();
    if !alpm_root_dir.starts_with(tmp_dir.to_str().unwrap()) {
        log::error!("alpm handle root at '{}' wasn't removed", alpm_root_dir);
        return Ok(());
    }

    // remove our temp repo dir
    fs::remove_dir_all(alpm_root_dir)?;

    Ok(())
}
