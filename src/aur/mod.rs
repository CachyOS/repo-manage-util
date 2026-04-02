mod dep_graph;

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use flate2::read::{GzDecoder, MultiGzDecoder};
use serde::Deserialize;
use tar::Archive;

// Our hardcoded client header
const USER_AGENT: &str = "repo-manage-util/1.0.0";

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder().user_agent(USER_AGENT).build().context("Failed to build HTTP client")
}

#[derive(Deserialize, Clone)]
pub struct Package {
    // AUR RPC search ReturnData
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "PackageBase")]
    pub package_base: String,

    #[serde(rename = "Version")]
    pub version: String,

    // AUR RPC info/multiinfo ReturnData
    #[serde(rename = "Depends", default)]
    pub depends: Vec<String>,

    #[serde(rename = "MakeDepends", default)]
    pub make_depends: Vec<String>,

    #[serde(rename = "OptDepends", default)]
    pub opt_depends: Vec<String>,

    #[serde(rename = "CheckDepends", default)]
    pub check_depends: Vec<String>,
}

pub struct PackageSummary {
    pub new_pkgs: Vec<Package>,
    pub build_order: Vec<Package>,
}

// Gets list of local AUR packages eligible for the update
pub async fn get_new_aur_pkgs(pkg_list: &[(String, String)]) -> Result<PackageSummary> {
    let aur_pkgs = fetch_snapshot().await?;

    let aur_map: HashMap<String, Package> =
        aur_pkgs.into_iter().map(|p| (p.name.clone(), p)).collect();
    let mut new_pkgs: Vec<Package> = vec![];

    for (name, local_ver) in pkg_list {
        if let Some(aur_pkg) = aur_map.get(name) {
            let local_ver = alpm::Version::new(local_ver.as_str());
            if alpm::Version::new(aur_pkg.version.as_str()) > local_ver {
                new_pkgs.push(aur_pkg.clone());
            }
        }
    }

    // calculate dep graph
    let targets: Vec<String> = new_pkgs.iter().map(|x| x.name.clone()).collect();
    let build_graph = dep_graph::build_dependency_graph(&targets, &aur_map)
        .context("Failed to build dep graph")?;
    let build_order =
        dep_graph::calculate_build_order(&build_graph).context("Failed to calc order")?;
    tracing::debug!("Build graph {build_graph:?}");

    // insert full package info to the build order
    let build_order = build_order
        .iter()
        .map(|x| aur_map.get(x).expect("how is that even possible").clone())
        .collect();

    // NOTE(vnepogodin): should we filter out packages which don't need update?

    Ok(PackageSummary { new_pkgs, build_order })
}

pub async fn pull_tarballs<PathLike: AsRef<Path>>(
    targets: &[Package],
    dest_path: PathLike,
) -> Result<()> {
    let mut pkgbases = targets.iter().map(|x| x.package_base.clone()).collect::<Vec<_>>();
    pkgbases.dedup();
    for pkgbase in &pkgbases {
        if pull_tarball(pkgbase, dest_path.as_ref()).await.is_ok() {
            continue;
        }

        // fallback to AUR Git
        tracing::debug!("Using Git fallback for {pkgbase}");
        pull_git_source(pkgbase, dest_path.as_ref())?;
    }

    Ok(())
}

async fn pull_tarball<PathLike: AsRef<Path>>(pkgbase: &str, dest_path: PathLike) -> Result<()> {
    tracing::debug!("Pulling '{pkgbase}'..");
    let url = format!("https://aur.archlinux.org/cgit/aur.git/snapshot/{pkgbase}.tar.gz");

    let retry_policy = reqwest::retry::for_host("aur.archlinux.org").max_retries_per_request(10);
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .retry(retry_policy)
        .build()
        .context("Failed to build client")?;
    let response = client.get(url).send().await?.error_for_status()?;

    let content = response.bytes().await?;
    let decoder = MultiGzDecoder::new(content.iter().as_slice());
    let mut archive = Archive::new(decoder);

    archive.unpack(&dest_path).context("Failed to unpack tarball")?;
    Ok(())
}

fn pull_git_source<PathLike: AsRef<Path>>(pkgbase: &str, dest_path: PathLike) -> Result<()> {
    tracing::debug!("Pulling Git '{pkgbase}'..");

    let git_path = dest_path.as_ref().join(pkgbase);
    if git_path.exists() {
        tracing::debug!("Removing existing source dir for {git_path:?}");
        fs::remove_dir_all(&git_path).context("Failed to remove existing source dir")?;
    }
    // keep only up to second parent
    pkg_manage_util::aur::clone_repo(pkgbase, git_path, Some(2i32), None)
        .context("Failed to fetch Git source")?;

    Ok(())
}

pub async fn fetch_snapshot() -> Result<Vec<Package>> {
    let url = "https://aur.archlinux.org/packages-meta-ext-v1.json.gz";
    let client = build_client()?;
    let response = client.get(url).send().await?.error_for_status()?;

    let content = response.bytes().await?;
    parse_search_snapshot_reader(content.iter().as_slice())
}

pub async fn fetch_packages() -> Result<Vec<String>> {
    let url = "https://aur.archlinux.org/packages.gz";
    let client = build_client()?;
    let response = client.get(url).send().await?.error_for_status()?;

    let content = response.bytes().await?;
    parse_lines_reader(content.iter().as_slice())
}

fn parse_search_snapshot_reader<R: std::io::Read>(reader: R) -> Result<Vec<Package>> {
    let reader = archive_reader(reader);
    let packages: Vec<Package> =
        serde_json::from_reader(reader).context("Failed to parse JSON from AUR snapshot")?;

    Ok(packages)
}

fn parse_lines_reader<R: std::io::Read>(reader: R) -> Result<Vec<String>> {
    let reader = archive_reader(reader);
    Ok(reader.lines().map_while(Result::ok).collect())
}

fn archive_reader<R: std::io::Read>(reader: R) -> BufReader<GzDecoder<R>> {
    let decoder = GzDecoder::new(reader);
    BufReader::new(decoder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    #[test]
    fn test_parse_search_gzipped_json() {
        let json_data = r#"[
        {"ID":1521974,"Name":"0wgram","PackageBaseID":204318,"PackageBase":"0wgram","Version":"1:1.4.0-1","Description":"Unofficial desktop version of Telegram messaging app","URL":"https://github.com/clansty/tdesktop","NumVotes":0,"Popularity":0.0,"OutOfDate":null,"Maintainer":"Clansty","Submitter":"Clansty","FirstSubmitted":1711800169,"LastModified":1723638864,"URLPath":"/cgit/aur.git/snapshot/0wgram.tar.gz","Depends":["hunspell","ffmpeg","hicolor-icon-theme"],"MakeDepends":["cmake","git"],"OptDepends":["webkit2gtk"],"License":["GPL3"]}
        ]"#;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json_data.as_bytes()).unwrap();
        let compressed_bytes = encoder.finish().unwrap();
        let result = parse_search_snapshot_reader(compressed_bytes.as_slice()).unwrap();

        assert_eq!(result.len(), 1);

        let pkg = &result[0];
        assert_eq!(pkg.name, "0wgram");
        assert_eq!(pkg.package_base, "0wgram");
        assert_eq!(pkg.version, "1:1.4.0-1");
        assert!(pkg.depends.contains(&"ffmpeg".to_string()));
        assert!(pkg.make_depends.contains(&"cmake".to_string()));
        assert!(pkg.opt_depends.contains(&"webkit2gtk".to_string()));
    }

    #[tokio::test]
    async fn test_parse_search_all_json() {
        let result = fetch_snapshot().await.unwrap();
        assert_ne!(result.len(), 0);
    }

    #[test]
    fn test_parse_lines() {
        let pkg_list = r#"1pass-git
        1password
        1password-beta
        1password-cli"#;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(pkg_list.as_bytes()).unwrap();
        let compressed_bytes = encoder.finish().unwrap();
        let result = parse_lines_reader(compressed_bytes.as_slice()).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result.as_slice(), pkg_list.lines().collect::<Vec<_>>());
    }
}
