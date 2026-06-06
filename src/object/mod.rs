use anyhow::Result;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;

use crate::util::compress_zlib;

pub(crate) mod blob;
pub(crate) mod commit;
pub(crate) mod tree;

pub trait GitObject {
    fn data(&self) -> &[u8];

    fn sha1(&self) -> Vec<u8> {
        Vec::from(&Sha1::digest(self.data())[..])
    }

    fn write_to_disk(&self) -> Result<()> {
        let obj_sha_hex = hex::encode(self.sha1());
        let (dir, filename) = obj_sha_hex.split_at(2);
        let dir_path = PathBuf::from(".git/objects/").join(dir);
        fs::create_dir_all(&dir_path)?;
        let path = dir_path.join(filename);
        fs::write(path, compress_zlib(self.data())?)?;

        Ok(())
    }
}
