use anyhow::{Result, ensure};

mod object;

use object::PackFileObject;

pub struct PackFile {
    pub version: u32,
    pub n_objects: u32,
    pub objects: Vec<PackFileObject>,
}

impl PackFile {
    pub fn try_new(data: &[u8]) -> Result<Self> {
        const IDENTIFIER: &[u8] = b"PACK";
        const IDENTIFIER_LEN: usize = IDENTIFIER.len();
        ensure!(
            &data[..IDENTIFIER_LEN] == IDENTIFIER,
            "invalid pack file: missing PACK signature"
        );

        const VERSION_LEN: usize = 4;
        const N_OBJECTS_LEN: usize = 4;

        let version =
            u32::from_be_bytes(data[IDENTIFIER_LEN..IDENTIFIER_LEN + VERSION_LEN].try_into()?);
        let n_objects = u32::from_be_bytes(
            data[IDENTIFIER_LEN + VERSION_LEN..IDENTIFIER_LEN + VERSION_LEN + N_OBJECTS_LEN]
                .try_into()?,
        );

        const HEADER_LEN: usize = IDENTIFIER_LEN + VERSION_LEN + N_OBJECTS_LEN;
        let mut offset = HEADER_LEN;
        let mut objects = Vec::with_capacity(n_objects as usize);

        for _ in 0..n_objects {
            let (consumed, obj) = PackFileObject::parse_next(&data[offset..], offset)?;
            offset += consumed;
            objects.push(obj);
        }
        dbg!(&offset);

        // TODO: verify checksum at data[offset..offset+20]

        Ok(Self {
            version,
            n_objects,
            objects,
        })
    }
}
