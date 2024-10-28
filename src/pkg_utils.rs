use crate::utils;

use std::collections::HashMap;
use std::path::Path;

type PackageMap = HashMap<String, Vec<(String, alpm::Version)>>;

pub fn get_debug_packages(pkg_list: &[String]) -> Vec<String> {
    // Identify debug packages from pkg list
    let mut debug_pkgs: Vec<String> = vec![];
    for pkg_filepath in pkg_list {
        let pkg_filename = Path::new(pkg_filepath).file_name().unwrap().to_str().unwrap();
        let pkg_name = crate::pkg_utils::get_pkgname_from_filename(pkg_filename);
        if pkg_name.ends_with("-debug") {
            debug_pkgs.push(pkg_filepath.clone());
        }
    }
    debug_pkgs.sort();

    debug_pkgs
}

pub fn get_outdated_pkgs(pkg_list: &[String]) -> Vec<String> {
    let mut pkg_map = get_pkgs_map(pkg_list);

    // Identify outdated packages for each group
    let mut outdated_pkgs: Vec<String> = vec![];
    for (_name, versions) in pkg_map.iter_mut() {
        if versions.len() > 1 {
            // Sort versions in descending order
            versions.sort_by(|a, b| b.1.vercmp(&a.1));

            // Add all but the latest version to the outdated list
            outdated_pkgs.extend(versions[1..].iter().map(|s| s.0.clone()));
        }
    }
    outdated_pkgs.sort();

    outdated_pkgs
}

// Remove outdated packages from pkg_list
pub fn remove_outdated_pkgs(pkg_list: &mut Vec<String>) {
    let outdated_pkgs = get_outdated_pkgs(pkg_list);
    pkg_list.retain(|pkg| !outdated_pkgs.contains(pkg));
}

// TODO(vnepogodin): add checking for new packages based on files in separate folder
// (profile.ref_folder) if very are newer package available in separate repo:
// 1. check if such exist and gather the list of them
// 2. copy those package files over to the repo directory
// 3. update the repo with newer packages
// 4. handle old packages which were present in the repository at the time, which we dumped before
pub fn get_new_pkgs(pkg_list: &[String]) -> Vec<String> {
    let mut pkg_map = get_pkgs_map(pkg_list);

    let mut new_pkgs: Vec<String> = vec![];
    for (_name, versions) in pkg_map.iter_mut() {
        if versions.len() > 1 {
            // Sort versions in descending order
            versions.sort_by(|a, b| b.1.vercmp(&a.1));

            // Add only the latest version to the list
            new_pkgs.push(versions[0].0.clone());
        }
    }
    new_pkgs.sort();

    new_pkgs
}

// Get list of packages with more than N versions
// NOTE: if the package has less than N versions, it will be ignored
pub fn get_stale_pkg_versions(pkg_list: &[String], n_versions: usize) -> PackageMap {
    let mut pkg_map = get_pkgs_map(pkg_list);

    let mut n_pkgs_map: PackageMap = HashMap::new();
    for (name, versions) in pkg_map.iter_mut() {
        if versions.len() > n_versions {
            // Sort versions in ascending order
            versions.sort_by(|a, b| a.1.vercmp(&b.1));

            // Get the amount of package versions which are lower than our threshold of N versions
            let diff_ver_num = versions.len() - n_versions;
            let pkg_versions = &versions[..diff_ver_num];
            n_pkgs_map.entry(name.into()).or_default().extend_from_slice(pkg_versions);
        }
    }

    n_pkgs_map
}

// Map of all packages in pkglist. where:
// (PKGNAME, [FILENAME of each VERSION])
fn get_pkgs_map(pkg_list: &[String]) -> PackageMap {
    let mut pkg_map: PackageMap = HashMap::new();

    // Group packages by name and store their versions
    for pkg_filepath in pkg_list {
        let pkg_filename = Path::new(pkg_filepath).file_name().unwrap().to_str().unwrap();
        let pkg_name = crate::pkg_utils::get_pkgname_from_filename(pkg_filename);
        let pkg_version = crate::pkg_utils::get_pkgver_from_filename(pkg_filename);

        let version = alpm::Version::new(pkg_version);

        pkg_map.entry(pkg_name.into()).or_default().push((pkg_filepath.clone(), version));
    }
    pkg_map
}

// pub fn get_pkgver_from_filename(filename: &str) -> &str {
// let range_size = filename.split('-').count();
// let dropped_range = range_size - 3;
//
// let first_pos = filename.match_indices('-').nth(dropped_range - 1).unwrap().0 + 1;
// let last_pos = filename.match_indices('-').nth(dropped_range + 1).unwrap().0;
// let last_pos = std::cmp::min(last_pos, filename.len());
//
// &filename[first_pos..last_pos]
// }

pub fn get_pkgname_from_filename(filename: &str) -> &str {
    let last_pos = filename.match_indices('-').nth_back(2).unwrap().0;
    &filename[..last_pos]
}

pub fn get_pkgver_from_filename(filename: &str) -> &str {
    let mut rng = filename.match_indices('-');
    let last_pos = rng.nth_back(0).unwrap().0;
    let first_pos = rng.nth_back(1).unwrap().0 + 1;

    &filename[first_pos..last_pos]
}

pub fn get_pkg_db_pair_from_path(file_path: &str) -> String {
    // NOTE: we can do here same as for pkgname and pkgver,
    // and just return &str which points to part of file_path
    let pkg_filename = Path::new(file_path).file_name().unwrap().to_str().unwrap();
    let pkg_name = get_pkgname_from_filename(pkg_filename);
    let pkg_ver = get_pkgver_from_filename(pkg_filename);

    format!("{pkg_name}-{pkg_ver}")
}

pub fn get_repo_db_prefix(repo_db_filename: &str) -> String {
    let repo_db_prefix =
        Path::new(repo_db_filename).file_stem().unwrap().to_str().unwrap().to_owned();
    if let Some(strpos) = repo_db_prefix.find(".db") {
        return utils::string_substr(&repo_db_prefix, 0, strpos).unwrap().into();
    }

    repo_db_prefix
}

pub fn remove_pkgs_without_sig(pkgs_list: &mut Vec<String>) {
    pkgs_list.retain(|pkg| {
        let pkg_sig_path = format!("{pkg}.sig");
        if !Path::new(&pkg_sig_path).exists() {
            log::error!("package doesn't have required signature {pkg}");
            false
        } else {
            true
        }
    });
}

pub fn replace_base_dir_for_pkgs(pkgs_list: &[String], base_dir: &Path) -> Vec<String> {
    pkgs_list
        .iter()
        .map(|file| {
            base_dir.join(Path::new(file).file_name().unwrap()).to_str().unwrap().to_owned()
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use crate::pkg_utils::*;

    #[test]
    fn test_pkgver_from_filename() {
        assert_eq!(
            get_pkgver_from_filename("mkinitcpio-nfs-utils-debug-0.3-8.1-x86_64.pkg.tar.zst"),
            "0.3-8.1"
        );
        assert_eq!(
            get_pkgver_from_filename("btrfs-progs-6.5.3-2.1-x86_64.pkg.tar.zst"),
            "6.5.3-2.1"
        );
        assert_eq!(
            get_pkgver_from_filename("octopi-dev-0.15.0.r4.b4301d7-1-x86_64.pkg.tar.zst"),
            "0.15.0.r4.b4301d7-1"
        );
        assert_eq!(
            get_pkgver_from_filename(
                "sayonara-player-git-1.8.0.beta1.r38.g3d444b4a-1-x86_64.pkg.tar.zst"
            ),
            "1.8.0.beta1.r38.g3d444b4a-1"
        );
        assert_eq!(
            get_pkgver_from_filename(
                "kvmtool-git-3.18.0.r1956.20230916.9cb1b46-1-x86_64.pkg.tar.zst"
            ),
            "3.18.0.r1956.20230916.9cb1b46-1"
        );
        assert_eq!(
            get_pkgver_from_filename(
                "linux-xanmod-linux-headers-bin-x64v3-6.6.8-1-x86_64.pkg.tar.zst"
            ),
            "6.6.8-1"
        );
        assert_eq!(
            get_pkgver_from_filename("bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst"),
            "3:1.11.0-1.1"
        );
        assert_eq!(
            get_pkgver_from_filename("argon2-20190702-5.1-x86_64.pkg.tar.zst"),
            "20190702-5.1"
        );
        assert_eq!(get_pkgver_from_filename("dash-0.5.12-1.1-x86_64.pkg.tar.zst"), "0.5.12-1.1");
    }

    #[test]
    fn test_pkgname_from_filename() {
        assert_eq!(
            get_pkgname_from_filename("mkinitcpio-nfs-utils-debug-0.3-8.1-x86_64.pkg.tar.zst"),
            "mkinitcpio-nfs-utils-debug"
        );
        assert_eq!(
            get_pkgname_from_filename("btrfs-progs-6.5.3-2.1-x86_64.pkg.tar.zst"),
            "btrfs-progs"
        );
        assert_eq!(
            get_pkgname_from_filename("octopi-dev-0.15.0.r4.b4301d7-1-x86_64.pkg.tar.zst"),
            "octopi-dev"
        );
        assert_eq!(
            get_pkgname_from_filename(
                "sayonara-player-git-1.8.0.beta1.r38.g3d444b4a-1-x86_64.pkg.tar.zst"
            ),
            "sayonara-player-git"
        );
        assert_eq!(
            get_pkgname_from_filename(
                "kvmtool-git-3.18.0.r1956.20230916.9cb1b46-1-x86_64.pkg.tar.zst"
            ),
            "kvmtool-git"
        );
        assert_eq!(
            get_pkgname_from_filename(
                "linux-xanmod-linux-headers-bin-x64v3-6.6.8-1-x86_64.pkg.tar.zst"
            ),
            "linux-xanmod-linux-headers-bin-x64v3"
        );
        assert_eq!(
            get_pkgname_from_filename("bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst"),
            "bcachefs-tools"
        );
        assert_eq!(get_pkgname_from_filename("argon2-20190702-5.1-x86_64.pkg.tar.zst"), "argon2");
        assert_eq!(get_pkgname_from_filename("dash-0.5.12-1.1-x86_64.pkg.tar.zst"), "dash");
        assert_eq!(get_pkgname_from_filename("emacs-29.4-2.1-x86_64.pkg.tar.zst"), "emacs");
        assert_eq!(
            get_pkgname_from_filename("emacs-nativecomp-29.4-2.1-x86_64.pkg.tar.zst"),
            "emacs-nativecomp"
        );
        assert_eq!(get_pkgname_from_filename("emacs-nox-29.4-2.1-x86_64.pkg.tar.zst"), "emacs-nox");
        assert_eq!(
            get_pkgname_from_filename("emacs-wayland-29.4-2.1-x86_64.pkg.tar.zst"),
            "emacs-wayland"
        );
    }

    #[test]
    fn test_pkg_db_pair_from_path() {
        assert_eq!(
            get_pkg_db_pair_from_path(
                "/to/file/mkinitcpio-nfs-utils-debug-0.3-8.1-x86_64.pkg.tar.zst"
            ),
            "mkinitcpio-nfs-utils-debug-0.3-8.1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path("/to/file/btrfs-progs-6.5.3-2.1-x86_64.pkg.tar.zst"),
            "btrfs-progs-6.5.3-2.1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path("/to/file/octopi-dev-0.15.0.r4.b4301d7-1-x86_64.pkg.tar.zst"),
            "octopi-dev-0.15.0.r4.b4301d7-1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path(
                "/to/file/sayonara-player-git-1.8.0.beta1.r38.g3d444b4a-1-x86_64.pkg.tar.zst"
            ),
            "sayonara-player-git-1.8.0.beta1.r38.g3d444b4a-1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path(
                "/to/file/kvmtool-git-3.18.0.r1956.20230916.9cb1b46-1-x86_64.pkg.tar.zst"
            ),
            "kvmtool-git-3.18.0.r1956.20230916.9cb1b46-1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path(
                "/to/file/linux-xanmod-linux-headers-bin-x64v3-6.6.8-1-x86_64.pkg.tar.zst"
            ),
            "linux-xanmod-linux-headers-bin-x64v3-6.6.8-1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path("bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst"),
            "bcachefs-tools-3:1.11.0-1.1"
        );
        assert_eq!(
            get_pkg_db_pair_from_path("/to/file/argon2-20190702-5.1-x86_64.pkg.tar.zst"),
            "argon2-20190702-5.1".to_owned()
        );
        assert_eq!(
            get_pkg_db_pair_from_path("dash-0.5.12-1.1-x86_64.pkg.tar.zst"),
            "dash-0.5.12-1.1".to_owned()
        );
    }

    #[test]
    fn test_repo_db_prefix() {
        assert_eq!(get_repo_db_prefix("example.db.tar.zst"), "example".to_owned());
        assert_eq!(get_repo_db_prefix("example.files.tar.zst"), "example.files.tar".to_owned());
    }

    #[test]
    fn test_n_pkg_version() {
        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.2-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
        ];
        let pkg_version_slice = get_stale_pkg_versions(&pkgs_list, 2);

        let expected_version_slice: PackageMap = HashMap::from([(
            "cachyos-cli-installer-new".to_string(),
            vec![(
                "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
                alpm::Version::new("0.7.0-1"),
            )],
        )]);

        assert_eq!(pkg_version_slice, expected_version_slice);

        let mut pkg_version_slice =
            get_stale_pkg_versions(&pkgs_list, 1).into_iter().collect::<Vec<_>>();
        pkg_version_slice.sort_by(|a, b| a.0.cmp(&b.0));

        let expected_version_slice =
            vec![
                (
                    "bcachefs-tools".to_string(),
                    vec![(
                        "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
                        alpm::Version::new("3:1.11.0-1.1"),
                    )],
                ),
                (
                    "cachyos-cli-installer-new".to_string(),
                    vec![
                (
                    "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
                    alpm::Version::new("0.7.0-1"),
                ),
                (
                    "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
                    alpm::Version::new("0.7.0-2"),
                ),
            ],
                ),
            ];

        assert_eq!(pkg_version_slice, expected_version_slice);
    }

    #[test]
    fn test_debug_pkgs() {
        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/bcachefs-tools-debug-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let debug_pkg_list = get_debug_packages(&pkgs_list);

        let expected_debug_pkg_list: Vec<String> =
            vec!["local_repo/x86_64/bcachefs-tools-debug-3:1.11.0-1.1-x86_64.pkg.tar.zst".into()];

        assert_eq!(debug_pkg_list, expected_debug_pkg_list);
    }

    #[test]
    fn test_outdated_pkgs() {
        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/bcachefs-tools-3:1.9.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let outdated_list = get_outdated_pkgs(&pkgs_list);

        let expected_outdated_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.9.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
        ];

        assert_eq!(outdated_list, expected_outdated_list);
    }

    #[test]
    fn test_fresh_pkgs() {
        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let outdated_list = get_outdated_pkgs(&pkgs_list);

        let expected_outdated_list: Vec<String> = vec![];
        assert_eq!(outdated_list, expected_outdated_list);
    }

    #[test]
    fn test_remove_outdated_pkgs() {
        let mut pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/bcachefs-tools-3:1.9.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let expected_pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];

        remove_outdated_pkgs(&mut pkgs_list);
        assert_eq!(pkgs_list, expected_pkgs_list);
    }
    #[test]
    fn test_remove_empty_outdated_pkgs() {
        let mut pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let expected_pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];

        remove_outdated_pkgs(&mut pkgs_list);
        assert_eq!(pkgs_list, expected_pkgs_list);
    }

    #[test]
    fn test_new_pkgs() {
        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/bcachefs-tools-3:1.9.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-2-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let new_pkgs_list = get_new_pkgs(&pkgs_list);

        let expected_new_pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
        ];

        assert_eq!(new_pkgs_list, expected_new_pkgs_list);
    }

    #[test]
    fn test_no_new_pkgs() {
        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];
        let new_pkgs_list = get_new_pkgs(&pkgs_list);

        let expected_new_pkgs_list: Vec<String> = vec![];
        assert_eq!(new_pkgs_list, expected_new_pkgs_list);
    }

    #[test]
    fn test_replace_base_dir_for_pkgs() {
        let base_dir = Path::new("local_repo_2/x86_64");

        let pkgs_list: Vec<String> = vec![
            "local_repo/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];

        let expected_pkgs_list: Vec<String> = vec![
            "local_repo_2/x86_64/bcachefs-tools-3:1.11.0-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo_2/x86_64/cachyos-cli-installer-new-0.7.0-3-x86_64.pkg.tar.zst".into(),
            "local_repo_2/x86_64/dolt-1.30.4-1.1-x86_64.pkg.tar.zst".into(),
            "local_repo_2/x86_64/dwl-git-0.2.1.r34.2d9740c-1-x86_64.pkg.tar.zst".into(),
            "local_repo_2/x86_64/dwm-6.2-4-x86_64.pkg.tar.zst".into(),
            "local_repo_2/x86_64/lightdm-webkit2-theme-arch-1:0.1-1-any.pkg.tar.zst".into(),
            "local_repo_2/x86_64/plymouth-theme-hud-3-git-r38.bf2f570-1-any.pkg.tar.zst".into(),
            "local_repo_2/x86_64/st-0.8.4-2-x86_64.pkg.tar.zst".into(),
        ];

        assert_eq!(replace_base_dir_for_pkgs(&pkgs_list, base_dir), expected_pkgs_list);
    }
}
