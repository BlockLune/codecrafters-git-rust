use anyhow::{Context, Result};
use bytes::Bytes;
use std::collections::HashMap;

use crate::util::pkt_line;

#[derive(Debug)]
pub struct GitRef {
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

    pub fn sha1(&self) -> &[u8] {
        &self.sha1
    }
}

#[derive(Debug)]
pub struct RefDiscovery {
    refs: HashMap<String, GitRef>,
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
            let ref_sha1_hex = std::str::from_utf8(sha1_hex_in_bytes)?;

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
            // pkt-line ref advertisements are text lines terminated by '\n'
            let ref_name = ref_name.trim_end();
            let git_ref = GitRef::try_new(ref_name, &ref_sha1_hex)?;
            refs.insert(ref_name.to_string(), git_ref);
        }

        Ok(Self { refs, capabilities })
    }

    pub fn head_sha1(&self) -> Result<&[u8]> {
        Ok(self
            .refs
            .get(&"HEAD".to_string())
            .context("HEAD not found")?
            .sha1())
    }

    pub fn branch_refs(&self) -> impl Iterator<Item = (&str, &[u8])> {
        const PREFIX: &str = "refs/heads/";
        self.refs
            .iter()
            .filter(|(name, _)| name.starts_with(PREFIX))
            .map(|(name, git_ref)| (name.as_str(), git_ref.sha1()))
    }

    pub fn default_branch(&self) -> Result<String> {
        for capability in &self.capabilities {
            if let Some(branch) = capability.strip_prefix("symref=HEAD:") {
                return Ok(branch.to_string());
            }
        }

        let head_sha1 = self.head_sha1()?;
        self.branch_refs()
            .find(|(_, sha1)| *sha1 == head_sha1)
            .map(|(name, _)| name.to_string())
            .context("could not determine default branch")
    }
}
