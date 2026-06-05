use anyhow::Result;

use crate::object::commit::CommitObject;

pub fn run(tree_sha: &str, parent_sha: &str, message: &str) -> Result<()> {
    let commit_obj = CommitObject::new(tree_sha, parent_sha, message);
    commit_obj.write_to_disk()?;
    println!("{}", hex::encode(commit_obj.sha1()));
    Ok(())
}
