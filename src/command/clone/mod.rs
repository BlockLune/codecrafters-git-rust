use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

mod client;
mod pack;
mod refs;

use crate::object::tree::parse_tree_entries;
use crate::util::{
    disk::{
        write_branch_ref,          //
        write_head_symref,         //
        write_remote_head_symref,  //
        write_remote_tracking_ref, //
    },
    get_decompressed_header_content_from_sha,
};
use client::GitApiClient;

const REMOTE_NAME: &str = "origin";

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let repo_url = canonicalize_repo_url(repo_url);
    let local_dir = PathBuf::from(resolve_local_dir(&repo_url, local_dir)?);

    let client = GitApiClient::new(&repo_url);
    let discovery = client.discover_refs().await?;

    let head_sha1 = discovery.head_sha1()?;
    let branch_refs = discovery.branch_refs().collect::<Vec<_>>();
    let mut want_refs = Vec::new();
    want_refs.push(head_sha1);
    for (_, sha1) in &branch_refs {
        want_refs.push(sha1);
    }

    let pack_file = client.fetch_pack_file(want_refs).await?;

    // write files to disk
    if local_dir.exists() {
        bail!(
            "fatal: destination path '{}' already exists.",
            local_dir.display()
        );
    }
    fs::create_dir_all(&local_dir)?;

    for object in pack_file.objects {
        object.write_to_disk(&local_dir)?;
    }

    let default_branch = discovery.default_branch()?;
    write_head_symref(&local_dir, &default_branch)?;
    write_branch_ref(&local_dir, &default_branch, head_sha1)?;
    write_remote_head_symref(&local_dir, REMOTE_NAME, &default_branch)?;
    for (ref_name, sha1) in &branch_refs {
        write_remote_tracking_ref(&local_dir, REMOTE_NAME, ref_name, sha1)?;
    }

    // checkout -- build the file tree from data in objects
    let head_sha1_hex = hex::encode(head_sha1);
    let (_, content) = get_decompressed_header_content_from_sha(&local_dir, &head_sha1_hex)?;
    let start_pos = 1 + content
        .iter()
        .position(|byte| *byte == b' ')
        .context("failed to parse head commit object")?;
    let end_pos = content
        .iter()
        .position(|byte| *byte == b'\n')
        .context("failed to parse head commit object")?;
    let tree_sha1_hex = std::str::from_utf8(&content[start_pos..end_pos])?;
    write_dir_for_tree(tree_sha1_hex, &local_dir, &local_dir)?;

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

fn write_dir_for_tree(tree_sha1_hex: &str, repo_root: &Path, current_dir: &Path) -> Result<()> {
    let (_, content) = get_decompressed_header_content_from_sha(repo_root, tree_sha1_hex)?;
    let tree_entries = parse_tree_entries(&content)?;
    for tree_entry in tree_entries {
        let name = std::str::from_utf8(&tree_entry.name)?;
        let path = current_dir.join(name);
        let sha1_hex = hex::encode(&tree_entry.sha1_20);

        if tree_entry.mode == b"40000" {
            // directory
            fs::create_dir_all(&path)?;
            write_dir_for_tree(&sha1_hex, repo_root, &path)?;
            continue;
        }

        let (_, content) = get_decompressed_header_content_from_sha(repo_root, &sha1_hex)?;

        #[cfg(unix)]
        if tree_entry.mode == b"120000" {
            // symlink
            let target = std::str::from_utf8(&content)?;
            std::os::unix::fs::symlink(target, &path)?;
            continue;
        }

        fs::write(&path, &content)?;

        #[cfg(unix)]
        if tree_entry.mode == b"100755" {
            // executable
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        }
    }

    Ok(())
}
