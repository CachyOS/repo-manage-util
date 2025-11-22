use std::collections::HashMap;
use std::io::{BufRead, BufReader};

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Package {
    // AUR RPC search ReturnData
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "PackageBase")]
    pub package_base: String,

    #[serde(rename = "Version")]
    pub version: String,

    #[serde(rename = "URLPath")]
    pub url_path: String,

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

// Gets list of local AUR packages eligible for the update
pub async fn get_new_aur_pkgs(pkg_list: &[(String, String)]) -> Result<Vec<Package>> {
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

    Ok(new_pkgs)
}

pub async fn fetch_snapshot() -> Result<Vec<Package>> {
    let url = "https://aur.archlinux.org/packages-meta-ext-v1.json.gz";
    let client = reqwest::Client::builder()
        .user_agent("repo-manage-util/1.0.0")
        .build()
        .context("Failed to build client")?;
    let response = client.get(url).send().await?.error_for_status()?;

    let content = response.bytes().await?;
    parse_search_snapshot_reader(content.iter().as_slice())
}

pub async fn fetch_packages() -> Result<Vec<String>> {
    let url = "https://aur.archlinux.org/packages.gz";
    let client = reqwest::Client::builder()
        .user_agent("repo-manage-util/1.0.0")
        .build()
        .context("Failed to build client")?;
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
