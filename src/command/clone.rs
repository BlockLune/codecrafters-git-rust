use anyhow::{Result, bail};

pub fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let local_dir = if local_dir.is_empty() {
        extract_local_dir_name_from_repo_url(repo_url)?
    } else {
        local_dir
    };

    Ok(())
}

fn extract_local_dir_name_from_repo_url(repo_url: &str) -> Result<&str> {
    if !repo_url.contains('/') {
        bail!("invalid repo_url");
    }
    let mut last_part = repo_url.split('/').last().unwrap();
    if last_part.ends_with(".git") {
        last_part = last_part.trim_end_matches(".git");
    }
    Ok(last_part)
}
