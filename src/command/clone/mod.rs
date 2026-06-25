use anyhow::{Result, bail};

mod client;
mod pack;
mod refs;

use client::GitApiClient;

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let repo_url = canonicalize_repo_url(repo_url);
    let local_dir = resolve_local_dir(&repo_url, local_dir)?;

    dbg!(&local_dir);

    let client = GitApiClient::new(&repo_url);
    let discovery = client.discover_refs().await?;

    let head_sha1 = discovery.head_sha1()?;
    let pack_file = client.fetch_pack_file(head_sha1).await?;

    dbg!(pack_file.version);
    dbg!(pack_file.n_objects);
    dbg!(pack_file.objects);

    // TODO: write pack_file to disk

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
