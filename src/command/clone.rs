use anyhow::{Result, bail};
use bytes::Bytes;

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let canonicalized_repo_url = canonicalize_repo_url(repo_url);
    let local_dir = if local_dir.is_empty() {
        extract_local_dir_from_canonicalized_repo_url(&canonicalized_repo_url)?
    } else {
        local_dir.to_string()
    };

    dbg!(&local_dir);

    let refs = get_refs(&canonicalized_repo_url).await?;

    dbg!(&refs);

    Ok(())
}

fn canonicalize_repo_url(repo_url: &str) -> String {
    let mut canonicalized = String::from(repo_url.trim_end_matches('/'));
    if !canonicalized.ends_with(".git") {
        canonicalized.push_str(".git");
    }
    canonicalized
}

fn extract_local_dir_from_canonicalized_repo_url(url: &str) -> Result<String> {
    if !url.contains('/') {
        bail!("invalid url");
    }
    let last_part = url
        .split('/')
        .last()
        .unwrap()
        .trim_end_matches(".git")
        .to_string();
    Ok(last_part)
}

async fn get_refs(repo_url: &str) -> Result<Bytes> {
    let url = format!("{}/info/refs?service=git-upload-pack", repo_url);
    let res = reqwest::get(&url).await?;
    Ok(res.bytes().await?)
}
