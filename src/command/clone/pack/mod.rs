use anyhow::{Result, ensure};
use std::collections::HashMap;

mod object;

use object::{RawPackObj, ResolvedPackObj, parse_next_raw_pack_obj};

pub struct PackFile {
    pub version: u32,
    pub n_objects: u32,
    pub objects: Vec<ResolvedPackObj>,
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
        let mut raw_objects: Vec<RawPackObj> = Vec::with_capacity(n_objects as usize);
        let mut resolved_objects: Vec<ResolvedPackObj> = Vec::with_capacity(n_objects as usize);

        for _ in 0..n_objects {
            let (raw_obj, consumed) = parse_next_raw_pack_obj(&data[offset..], offset)?;
            offset += consumed;
            raw_objects.push(raw_obj);
        }

        // given one offset, get the index of the raw object in raw_objects
        let mut offset_to_raw_index = HashMap::new();
        for (i, raw_obj) in raw_objects.iter().enumerate() {
            offset_to_raw_index.insert(raw_obj.offset(), i);
        }

        // given the sha1, get the index of the raw object in raw_objects
        let mut sha1_to_raw_index = HashMap::new();

        // resolve objects
        let mut resolved_cache = HashMap::new();
        // first iteration we resolve all base and ofs_delta objects
        for (i, raw_obj) in raw_objects.iter().enumerate() {
            if raw_obj.is_ref_delta() {
                continue;
            }
            // we also update sha1_to_raw_index here
            let resolved = ResolvedPackObj::try_from_raw(
                i,
                &raw_objects,
                &offset_to_raw_index,
                &mut sha1_to_raw_index,
                &mut resolved_cache,
            )?;
            resolved_objects.push(resolved);
        }
        // second iteration we focus on ref_delta objects
        for (i, raw_obj) in raw_objects.iter().enumerate() {
            if !raw_obj.is_ref_delta() {
                continue;
            }
            let resolved = ResolvedPackObj::try_from_raw(
                i,
                &raw_objects,
                &offset_to_raw_index,
                &mut sha1_to_raw_index,
                &mut resolved_cache,
            )?;
            resolved_objects.push(resolved);
        }

        // TODO: verify checksum at data[offset..offset+20]

        Ok(Self {
            version,
            n_objects,
            objects: resolved_objects,
        })
    }
}
