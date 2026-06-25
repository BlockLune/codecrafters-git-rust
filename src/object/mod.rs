use anyhow::Result;
use sha1::{Digest, Sha1};
use std::path::PathBuf;

use crate::util::disk::write_to_disk;

pub(crate) mod blob;
pub(crate) mod commit;
pub(crate) mod tree;

pub trait GitObject {
    fn data(&self) -> &[u8];

    fn sha1(&self) -> Vec<u8> {
        Vec::from(&Sha1::digest(self.data())[..])
    }

    fn write_to_disk(&self) -> Result<()> {
        write_to_disk(&PathBuf::from("."), &self.sha1(), self.data())
    }
}
