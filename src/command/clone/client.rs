use anyhow::{Result, ensure};
use bytes::Bytes;

use crate::util::pkt_line;

use super::pack::PackFile;
use super::refs::RefDiscovery;

#[derive(Debug)]
pub struct GitApiClient {
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

    pub async fn fetch_pack_file(
        &self,
        want_sha1s: impl IntoIterator<Item = &[u8]>,
    ) -> Result<PackFile> {
        let mut want_pkts = String::new();
        for want_sha1 in want_sha1s {
            let want_payload = format!("want {}\n", hex::encode(want_sha1));
            let want_pkt = pkt_line::encode(&want_payload);
            want_pkts.push_str(&want_pkt);
        }
        let done_pkt = pkt_line::encode("done\n");
        let body = Bytes::from(format!("{}0000{}", want_pkts, done_pkt));

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
        ensure!(
            &data[..EXPECTED_NAK_LINE_LEN] == EXPECTED_NAK_LINE,
            "unexpected response: expected NAK"
        );

        let pack_file_data = &data[EXPECTED_NAK_LINE_LEN..];
        let pack_file = PackFile::try_new(pack_file_data)?;

        Ok(pack_file)
    }
}
