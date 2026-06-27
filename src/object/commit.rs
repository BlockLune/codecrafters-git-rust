use anyhow::{Result, bail};

use crate::object::GitObject;

pub struct CommitObject(Vec<u8>);

static HARDCODE_AUTHOR: &str = "John Doe <john@example.com> 1234567890 +0000";

impl CommitObject {
    pub fn try_new(tree_sha_hex: &str, parent_sha_hex: &str, commit_msg: &str) -> Result<Self> {
        if tree_sha_hex.is_empty() {
            bail!("aborting commit due to empty tree sha");
        }
        if commit_msg.is_empty() {
            bail!("aborting commit due to empty commit message");
        }

        let mut content = Vec::from(format!("tree {}\n", tree_sha_hex).as_bytes());

        if !parent_sha_hex.is_empty() {
            content.extend_from_slice(format!("parent {}\n", parent_sha_hex).as_bytes());
        }

        content.extend_from_slice(format!("author {}\n", HARDCODE_AUTHOR).as_bytes());
        content.extend_from_slice(format!("committer {}\n", HARDCODE_AUTHOR).as_bytes());
        content.push(b'\n');
        content.extend_from_slice(format!("{}\n", commit_msg).as_bytes());

        let mut data = Vec::from(format!("commit {}\0", content.len()).as_bytes());
        data.extend_from_slice(&content);

        Ok(Self(data))
    }
}

impl GitObject for CommitObject {
    fn data(&self) -> &[u8] {
        &self.0
    }
}
