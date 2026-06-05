use anyhow::Result;

use crate::utils::{compute_sha1, write_obj_to_disk};

pub struct TreeEntry {
    #[allow(unused)]
    pub mode: Vec<u8>,
    pub name: Vec<u8>,
    pub sha1_20: Vec<u8>,
}

impl TreeEntry {
    pub fn new(mode: &[u8], name: &[u8], sha: &[u8]) -> Self {
        Self {
            mode: Vec::from(mode),
            name: Vec::from(name),
            sha1_20: Vec::from(&sha[..20]),
        }
    }
}

pub struct TreeObject(Vec<u8>);

impl TreeObject {
    pub fn new(entries: &[TreeEntry]) -> Self {
        let mut content: Vec<u8> = Vec::new();
        for entry in entries {
            content.extend_from_slice(&entry.name);
            content.push(b' ');
            content.extend_from_slice(&entry.name);
            content.push(b'\0');
            content.extend_from_slice(&entry.sha1_20);
        }

        let mut data = Vec::from(format!("tree {}\0", content.len()).as_bytes());
        data.extend_from_slice(&content);

        Self(data)
    }

    pub fn sha1(&self) -> Vec<u8> {
        compute_sha1(&self.0)
    }

    pub fn write_to_disk(&self) -> Result<()> {
        write_obj_to_disk(&self.sha1(), &self.0)
    }
}
