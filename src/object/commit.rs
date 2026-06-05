use anyhow::Result;

use crate::utils::{compute_sha1, write_obj_to_disk};

pub struct CommitObject(Vec<u8>);

static HARDCODE_AUTHOR: &str = "John Doe <john@example.com> 1234567890 +0000";

impl CommitObject {
    pub fn new(tree_sha_hex: &str, parent_sha_hex: &str, commit_msg: &str) -> Self {
        let mut content = Vec::from(format!("tree {}\n", tree_sha_hex).as_bytes());
        content.extend_from_slice(format!("parent {}\n", parent_sha_hex).as_bytes());
        content.extend_from_slice(format!("author {}\n", HARDCODE_AUTHOR).as_bytes());
        content.extend_from_slice(format!("committer {}\n", HARDCODE_AUTHOR).as_bytes());
        content.push(b'\n');
        content.extend_from_slice(commit_msg.as_bytes());

        let mut data = Vec::from(format!("commit {}\0", content.len()).as_bytes());
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
