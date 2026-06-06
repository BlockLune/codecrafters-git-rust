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

    let refs_data = get_refs_data(&canonicalized_repo_url).await?;
    let payloads = parse_payloads(refs_data)?;

    dbg!(&payloads);

    let mut git_refs = Vec::new();
    let mut compatibilities = Vec::new();
    for payload in payloads.iter().skip(1) {
        let sha1_hex_in_bytes = &payload[..40];
        let rest = &payload[41..];
        let ref_sha1_hex = String::from_utf8_lossy(sha1_hex_in_bytes);

        let ref_name;
        if let Some((pos, _)) = rest.iter().enumerate().find(|&(_, byte)| *byte == b'\0') {
            let ref_name_in_bytes = &rest[..pos];
            let compatibilities_in_bytes = &rest[pos + 1..];
            ref_name = String::from_utf8_lossy(ref_name_in_bytes);
            let compatibilities_string = String::from_utf8_lossy(compatibilities_in_bytes)
                .trim()
                .to_string();
            compatibilities = compatibilities_string
                .split_whitespace()
                .map(String::from)
                .collect();
        } else {
            ref_name = String::from_utf8_lossy(rest);
        }
        let git_ref = GitRef::try_new(&ref_name, &ref_sha1_hex)?;
        git_refs.push(git_ref);
    }

    dbg!(&compatibilities);
    dbg!(&git_refs);

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

async fn get_refs_data(repo_url: &str) -> Result<Bytes> {
    let url = format!("{}/info/refs?service=git-upload-pack", repo_url);
    let res = reqwest::get(&url).await?;
    Ok(res.bytes().await?)
}

fn parse_payloads(data: Bytes) -> Result<Vec<Bytes>> {
    let mut payloads = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let length_hex_string = String::from_utf8_lossy(&data[i..i + 4]);
        let length = usize::from_str_radix(&length_hex_string, 16)?;
        if length == 0 {
            i += 4;
            continue;
        }
        let payload = Bytes::copy_from_slice(&data[i + 4..i + length]);
        payloads.push(payload);
        i += length;
    }
    Ok(payloads)
}

#[derive(Debug)]
struct GitRef {
    name: String,
    sha1: Bytes,
}

impl GitRef {
    pub fn try_new(name: &str, sha1_hex: &str) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            sha1: Bytes::from(hex::decode(sha1_hex)?),
        })
    }
}
