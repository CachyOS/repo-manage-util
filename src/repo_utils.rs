use crate::config;

use anyhow::Result;
use subprocess::{Exec, Redirection};

// Calls repo-add on provided repo and package files
pub fn handle_repo_add(profile: &config::Profile, pkgfiles: &[String]) -> Result<()> {
    let mut repo_add_args = profile.add_params.clone();
    repo_add_args.push(profile.repo.clone());

    // push provided package files into repo-add args
    repo_add_args.extend_from_slice(pkgfiles);
    log::debug!("repo_add_args := {repo_add_args:?}");

    let output = Exec::cmd("repo-add")
        .args(&repo_add_args)
        .stderr(Redirection::Merge)
        .stdout(Redirection::Pipe)
        .capture()?;

    let proc_output = String::from_utf8_lossy(&output.stdout);
    if !output.success() {
        log::error!("repo-add output:\n{proc_output}");
        anyhow::bail!("repo-add failed!");
    }
    log::debug!("repo-add output:\n{proc_output}");

    Ok(())
}

// Calls repo-remove on provided repo and package names
pub fn handle_repo_remove(profile: &config::Profile, pkgname_list: &[String]) -> Result<()> {
    let mut repo_remove_args = profile.rm_params.clone();
    repo_remove_args.push(profile.repo.clone());

    // push provided package names into repo-remove args
    repo_remove_args.extend_from_slice(pkgname_list);
    log::debug!("repo_remove_args := {repo_remove_args:?}");

    let output = Exec::cmd("repo-remove")
        .args(&repo_remove_args)
        .stderr(Redirection::Merge)
        .stdout(Redirection::Pipe)
        .capture()?;

    let proc_output = String::from_utf8_lossy(&output.stdout);
    if !output.success() {
        log::error!("repo-remove output:\n{proc_output}");
        anyhow::bail!("repo-remove failed!");
    }
    log::debug!("repo-remove output:\n{proc_output}");

    Ok(())
}
