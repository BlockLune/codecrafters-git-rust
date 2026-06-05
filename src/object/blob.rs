use anyhow::Result;

use crate::utils::{compute_sha1, write_obj_to_disk};

pub struct BlobObject(Vec<u8>);

impl BlobObject {
    pub fn new(file_content: &[u8]) -> Self {
        let mut data = Vec::from(format!("blob {}\0", file_content.len()).as_bytes());
        data.extend_from_slice(&file_content);
        BlobObject(data)
    }

    pub fn sha1(&self) -> Vec<u8> {
        compute_sha1(&self.0)
    }

    pub fn write_to_disk(&self) -> Result<()> {
        write_obj_to_disk(&self.sha1(), &self.0)
    }
}
