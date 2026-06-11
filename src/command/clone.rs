use anyhow::{Context, Result, bail};
use bytes::Bytes;
use std::collections::HashMap;

use crate::util::pkt_line;

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let repo_url = canonicalize_repo_url(repo_url);
    let local_dir = resolve_local_dir(&repo_url, local_dir);

    dbg!(&local_dir);

    let client = GitApiClient::new(&repo_url);
    let discovery = client.discover_refs().await?;

    let head_sha1 = discovery.head_sha1()?;
    let pack = client.fetch_pack(head_sha1).await?;

    dbg!(&pack);

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

#[derive(Debug)]
struct GitApiClient {
    client: reqwest::Client,
    repo_url: String,
}

impl GitApiClient {
    pub fn new(repo_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            repo_url: repo_url.to_string(),
        }
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.repo_url, path);
        self.client.get(&url)
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.repo_url, path);
        self.client.post(&url)
    }

    pub async fn discover_refs(&self) -> Result<RefDiscovery> {
        let res = self
            .get("info/refs?service=git-upload-pack")
            .send()
            .await?
            .error_for_status()?;
        let discovery = RefDiscovery::parse(res.bytes().await?)?;
        Ok(discovery)
    }

    pub async fn fetch_pack(&self, head_sha1: &[u8]) -> Result<Bytes> {
        let want_payload = format!("want {}\n", hex::encode(head_sha1));

        let want_pkt = pkt_line::encode(&want_payload);
        let done_pkt = pkt_line::encode("done\n");

        let body = Bytes::from(format!("{}0000{}", want_pkt, done_pkt));

        let res = self
            .post("git-upload-pack")
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(body)
            .send()
            .await?
            .error_for_status()?;

        Ok(res.bytes().await?)
    }
}

#[derive(Debug)]
struct GitRef {
    #[allow(unused)]
    name: String,
    sha1: Vec<u8>,
}

impl GitRef {
    pub fn try_new(name: &str, sha1_hex: &str) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            sha1: hex::decode(sha1_hex)?,
        })
    }

    pub fn sha1(&self) -> &Vec<u8> {
        &self.sha1
    }
}

struct RefDiscovery {
    refs: HashMap<String, GitRef>,
    #[allow(unused)]
    capabilities: Vec<String>,
}

impl RefDiscovery {
    pub fn parse(data: Bytes) -> Result<Self> {
        let payloads = pkt_line::decode(&data)?;

        let mut refs = HashMap::new();
        let mut capabilities = Vec::new();

        for payload in payloads.iter().skip(1) {
            const SHA1_HEX_LEN_BYTES: usize = 40;

            let sha1_hex_in_bytes = &payload[..SHA1_HEX_LEN_BYTES];
            let rest = &payload[SHA1_HEX_LEN_BYTES + 1..];
            let ref_sha1_hex = String::from_utf8_lossy(sha1_hex_in_bytes);

            let ref_name;
            if let Some((pos, _)) = rest.iter().enumerate().find(|&(_, byte)| *byte == b'\0') {
                let ref_name_in_bytes = &rest[..pos];
                let capabilities_in_bytes = &rest[pos + 1..];
                ref_name = std::str::from_utf8(ref_name_in_bytes)?;
                let capabilities_string = std::str::from_utf8(capabilities_in_bytes)?.trim();
                capabilities = capabilities_string
                    .split_whitespace()
                    .map(String::from)
                    .collect();
            } else {
                ref_name = std::str::from_utf8(rest)?;
            }
            let git_ref = GitRef::try_new(ref_name, &ref_sha1_hex)?;
            refs.insert(ref_name.to_string(), git_ref);
        }

        Ok(Self { refs, capabilities })
    }

    pub fn head_sha1(&self) -> Result<&Vec<u8>> {
        Ok(self
            .refs
            .get(&"HEAD".to_string())
            .context("HEAD not found")?
            .sha1())
    }

    #[allow(unused)]
    pub fn symref_head(&self) -> Option<String> {
        for capability in &self.capabilities {
            if capability.starts_with("symref=HEAD:") {
                return Some(capability.trim_start_matches("symref=HEAD:").to_string());
            }
        }
        None
    }
}
