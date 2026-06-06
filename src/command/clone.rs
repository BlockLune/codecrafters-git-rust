use anyhow::{Context, Result, bail};
use bytes::Bytes;
use std::collections::HashMap;

use crate::util::pkt_line;

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let canonicalized_repo_url = canonicalize_repo_url(repo_url);
    let local_dir = if local_dir.is_empty() {
        extract_local_dir_from_canonicalized_repo_url(&canonicalized_repo_url)?
    } else {
        local_dir.to_string()
    };

    dbg!(&local_dir);

    let refs_data = get_refs_data(&canonicalized_repo_url).await?;
    let ref_discovery = RefDiscovery::parse(refs_data)?;
    let head_sha1_hex = hex::encode(ref_discovery.head_sha1()?);
    let want_payload = format!("want {}\n", head_sha1_hex);
    let want_pkt = pkt_line::encode(&want_payload);
    let done_pkt = pkt_line::encode("done\n");
    let body = Bytes::from(format!("{}0000{}", want_pkt, done_pkt));

    dbg!(&body);

    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/git-upload-pack", canonicalized_repo_url))
        .header("Content-Type", "application/x-git-upload-pack-request")
        .body(body)
        .send()
        .await?;

    dbg!(&res);

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

    pub fn sha1(&self) -> Bytes {
        self.sha1.clone()
    }
}

fn find_symref_head(compatibilites: &Vec<String>) -> Option<String> {
    for compatibility in compatibilites {
        if compatibility.starts_with("symref=HEAD:") {
            return Some(compatibility.trim_start_matches("symref=HEAD:").to_string());
        }
    }
    None
}

struct RefDiscovery {
    refs: HashMap<String, GitRef>,
    symref_head: String,
    compatibilities: Vec<String>,
}

impl RefDiscovery {
    pub fn parse(data: Bytes) -> Result<Self> {
        let payloads = pkt_line::decode(data)?;
        let mut git_refs = HashMap::new();
        let mut compatibilities = Vec::new();
        for payload in payloads.iter().skip(1) {
            const SHA1_HEX_LEN_BYTES: usize = 40;

            let sha1_hex_in_bytes = &payload[..SHA1_HEX_LEN_BYTES];
            let rest = &payload[SHA1_HEX_LEN_BYTES + 1..];
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
            git_refs.insert(ref_name.to_string(), git_ref);
        }
        let symref_head = find_symref_head(&compatibilities).context("`symref=HEAD:` not found")?;

        Ok(Self {
            refs: git_refs,
            symref_head,
            compatibilities,
        })
    }

    pub fn head_sha1(&self) -> Result<Bytes> {
        Ok(self
            .refs
            .get(&"HEAD".to_string())
            .context("HEAD not found")?
            .sha1())
    }
}
