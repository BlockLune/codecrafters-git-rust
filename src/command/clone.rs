use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use std::collections::HashMap;
use std::convert::TryFrom;

use crate::util::compression::decompress_zlib;
use crate::util::pkt_line;

pub async fn run(repo_url: &str, local_dir: &str) -> Result<()> {
    let repo_url = canonicalize_repo_url(repo_url);
    let local_dir = resolve_local_dir(&repo_url, local_dir);

    dbg!(&local_dir);

    let client = GitApiClient::new(&repo_url);
    let discovery = client.discover_refs().await?;

    let head_sha1 = discovery.head_sha1()?;
    let pack_file = client.fetch_pack_file(head_sha1).await?;

    dbg!(pack_file.version);
    dbg!(pack_file.n_objects);

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

    pub async fn fetch_pack_file(&self, head_sha1: &[u8]) -> Result<PackFile> {
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

        let data = res.bytes().await?;

        const EXPECTED_NAK_LINE: &[u8] = b"0008NAK\n";
        const EXPECTED_NAK_LINE_LEN: usize = EXPECTED_NAK_LINE.len();
        assert_eq!(&data[..EXPECTED_NAK_LINE_LEN], EXPECTED_NAK_LINE);

        let pack_file_data = &data[EXPECTED_NAK_LINE_LEN..];
        let pack_file = PackFile::try_new(pack_file_data)?;

        Ok(pack_file)
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

struct PackFile {
    pub version: u32,
    pub n_objects: u32,
    pub objects: Vec<PackFileObject>,
}

impl PackFile {
    pub fn try_new(data: &[u8]) -> Result<Self> {
        const IDENTIFIER: &[u8] = b"PACK";
        const IDENTIFIER_LEN: usize = IDENTIFIER.len();
        assert_eq!(&data[..IDENTIFIER_LEN], IDENTIFIER);

        const VERSION_LEN: usize = 4;
        const N_OBJECTS_LEN: usize = 4;

        let version =
            u32::from_be_bytes(data[IDENTIFIER_LEN..IDENTIFIER_LEN + VERSION_LEN].try_into()?);
        let n_objects = u32::from_be_bytes(
            data[IDENTIFIER_LEN + VERSION_LEN..IDENTIFIER_LEN + VERSION_LEN + N_OBJECTS_LEN]
                .try_into()?,
        );

        const HEADER_LEN: usize = IDENTIFIER_LEN + VERSION_LEN + N_OBJECTS_LEN;
        let mut offset = HEADER_LEN;
        let mut objects = Vec::with_capacity(n_objects as usize);

        for _ in 0..n_objects {
            let (consumed, obj) = PackFileObject::parse_next(&data[offset..])?;
            offset += consumed;
            objects.push(obj);
        }
        dbg!(&offset);

        // TODO: verify checksum at data[offset..offset+20]

        Ok(Self {
            version,
            n_objects,
            objects,
        })
    }
}

#[derive(Debug)]
enum ObjectType {
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    OfsDelta = 6,
    RefDelta = 7,
}

impl TryFrom<u8> for ObjectType {
    type Error = String;

    fn try_from(value: u8) -> std::prelude::v1::Result<Self, Self::Error> {
        match value {
            1 => Ok(ObjectType::Commit),
            2 => Ok(ObjectType::Tree),
            3 => Ok(ObjectType::Blob),
            4 => Ok(ObjectType::Tag),
            6 => Ok(ObjectType::OfsDelta),
            7 => Ok(ObjectType::RefDelta),
            _ => Err(format!("invalid: {}", value)),
        }
    }
}

#[derive(Debug)]
struct PackFileObject {
    pub obj_type: ObjectType,
    pub obj_size: usize,
    pub content: Vec<u8>,
}

impl PackFileObject {
    pub fn parse_next(data: &[u8]) -> Result<(usize, Self)> {
        let first_byte = data[0];
        let obj_type =
            ObjectType::try_from((first_byte >> 4) & 0b111).map_err(|e| anyhow!("{}", e))?;

        // MSB
        let mut obj_size = (first_byte & 0b1111) as usize;
        let mut idx = 1;
        let mut shift = 4;
        while idx < data.len() && (data[idx - 1] & 0b10000000) != 0 {
            obj_size |= ((data[idx] & 0b01111111) as usize) << shift;
            shift += 7;
            idx += 1;
        }
        let header_len = idx;

        match obj_type {
            ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                let (decompressed, compressed_len) = decompress_zlib(&data[header_len..])?;
                Ok((
                    header_len + compressed_len,
                    Self {
                        obj_type,
                        obj_size,
                        content: decompressed,
                    },
                ))
            }
            ObjectType::OfsDelta => {
                let (offset, compressed_delta_data) = parse_ofs_delta(&data[header_len..]);
                let delta_data = decompress_zlib(compressed_delta_data)?;
                todo!();
            }
            ObjectType::RefDelta => {
                let (base_sha1, compressed_delta_data) = parse_ref_delta(&data[header_len..]);
                let delta_data = decompress_zlib(compressed_delta_data)?;
                todo!();
            }
        }
    }
}

fn parse_ofs_delta(data: &[u8]) -> (usize, &[u8]) {
    let mut offset: usize = (data[0] & 0b01111111) as usize;
    let mut i: usize = 1;
    while i < data.len() && (data[i - 1] & 0b10000000) != 0 {
        offset = (offset + 1) << 7 | (data[i] & 0b01111111) as usize;
        i += 1;
    }
    (offset, &data[i..])
}

fn parse_ref_delta(data: &[u8]) -> ([u8; 20], &[u8]) {
    let base_sha1: [u8; 20] = data[..20].try_into().unwrap();
    (base_sha1, &data[20..])
}
