pub struct TreeEntry {
    pub mode: Vec<u8>,
    pub name: Vec<u8>,
    pub sha_raw_20: Vec<u8>,
}

impl TreeEntry {
    pub fn new(mode: &[u8], name: &[u8], sha_raw: &[u8]) -> Self {
        Self {
            mode: Vec::from(mode),
            name: Vec::from(name),
            sha_raw_20: Vec::from(&sha_raw[..20]),
        }
    }
}

pub struct TreeObject {
}
