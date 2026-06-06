use crate::object::GitObject;

pub struct BlobObject(Vec<u8>);

impl BlobObject {
    pub fn new(file_content: &[u8]) -> Self {
        let mut data = Vec::from(format!("blob {}\0", file_content.len()).as_bytes());
        data.extend_from_slice(&file_content);
        Self(data)
    }
}

impl GitObject for BlobObject {
    fn data(&self) -> &[u8] {
        &self.0
    }
}
