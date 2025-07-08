use crate::config;

use anyhow::Result;
use subprocess::{Exec, Redirection};

// Calls repo-add on provided repo and package files
pub fn handle_repo_add(profile: &config::Profile, pkgfiles: &[String]) -> Result<()> {
    let mut repo_add_args = profile.add_params.clone();
    repo_add_args.push(profile.repo.clone());

    // push provided package files into repo-add args
    repo_add_args.extend_from_slice(pkgfiles);
    tracing::debug!("repo_add_args := {repo_add_args:?}");

    let output = Exec::cmd("repo-add")
        .args(&repo_add_args)
        .stderr(Redirection::Merge)
        .stdout(Redirection::Pipe)
        .capture()?;

    let proc_output = String::from_utf8_lossy(&output.stdout);
    if !output.success() {
        tracing::error!("repo-add output:\n{proc_output}");
        anyhow::bail!("repo-add failed!");
    }
    tracing::debug!("repo-add output:\n{proc_output}");

    Ok(())
}

// Calls repo-remove on provided repo and package names
pub fn handle_repo_remove(profile: &config::Profile, pkgname_list: &[String]) -> Result<()> {
    let mut repo_remove_args = profile.rm_params.clone();
    repo_remove_args.push(profile.repo.clone());

    // push provided package names into repo-remove args
    repo_remove_args.extend_from_slice(pkgname_list);
    tracing::debug!("repo_remove_args := {repo_remove_args:?}");

    let output = Exec::cmd("repo-remove")
        .args(&repo_remove_args)
        .stderr(Redirection::Merge)
        .stdout(Redirection::Pipe)
        .capture()?;

    let proc_output = String::from_utf8_lossy(&output.stdout);
    if !output.success() {
        tracing::error!("repo-remove output:\n{proc_output}");
        anyhow::bail!("repo-remove failed!");
    }
    tracing::debug!("repo-remove output:\n{proc_output}");

    Ok(())
}
