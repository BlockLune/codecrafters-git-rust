use anyhow::{Result, bail};
use std::path::PathBuf;

mod client;
mod pack;
mod refs;

use crate::util::disk::{
    write_branch_ref, write_head_symref, write_remote_head_symref, write_remote_tracking_ref,
};
use client::GitApiClient;

const REMOTE_NAME: &str = "origin";

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let repo_url = canonicalize_repo_url(repo_url);
    let local_dir = PathBuf::from(resolve_local_dir(&repo_url, local_dir)?);

    let client = GitApiClient::new(&repo_url);
    let discovery = client.discover_refs().await?;

    let head_sha1 = discovery.head_sha1()?;
    let pack_file = client.fetch_pack_file(head_sha1).await?;

    for object in pack_file.objects {
        object.write_to_disk(&local_dir)?;
    }
    if let Some(symref_head) = discovery.symref_head() {
        write_head_symref(&local_dir, &symref_head)?;
        write_branch_ref(&local_dir, &symref_head, head_sha1)?;
        write_remote_head_symref(&local_dir, REMOTE_NAME, &symref_head)?;
    }
    for (ref_name, git_ref) in discovery.refs() {
        const PREFIX: &str = "refs/heads/";
        if !ref_name.starts_with(PREFIX) {
            continue;
        }
        write_remote_tracking_ref(&local_dir, REMOTE_NAME, ref_name, git_ref.sha1())?;
    }

    Ok(())
}

fn canonicalize_repo_url(repo_url: &str) -> String {
    let mut canonicalized = String::from(repo_url.trim_end_matches('/'));
    if !canonicalized.ends_with(".git") {
        canonicalized.push_str(".git");
    }
    canonicalized
}

fn resolve_local_dir(repo_url: &str, local_dir: &str) -> Result<String> {
    if !local_dir.is_empty() {
        return Ok(local_dir.to_string());
    }

    if !repo_url.contains('/') {
        bail!("invalid url");
    }

    Ok(repo_url
        .split('/')
        .last()
        .unwrap()
        .trim_end_matches(".git")
        .to_string())
}
