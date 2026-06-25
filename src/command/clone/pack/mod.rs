use anyhow::{Result, ensure};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};

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
        let mut pending_raw_indexes = HashSet::new();
        // first iteration we try to resolve any base and ofs_delta objects that can be resolved
        for (i, raw_obj) in raw_objects.iter().enumerate() {
            if raw_obj.is_ref_delta() {
                pending_raw_indexes.insert(i);
                continue;
            }
            // we also update sha1_to_raw_index here
            if let Some(resolved) = ResolvedPackObj::try_from_raw(
                i,
                &raw_objects,
                &offset_to_raw_index,
                &mut sha1_to_raw_index,
                &mut resolved_cache,
            )? {
                resolved_objects.push(resolved);
            } else {
                pending_raw_indexes.insert(i);
            }
        }
        // keep retrying pending objects until every resolvable object is resolved
        while !pending_raw_indexes.is_empty() {
            let mut resolved_something = false;
            for i in pending_raw_indexes.clone().into_iter() {
                if let Some(resolved) = ResolvedPackObj::try_from_raw(
                    i,
                    &raw_objects,
                    &offset_to_raw_index,
                    &mut sha1_to_raw_index,
                    &mut resolved_cache,
                )? {
                    resolved_objects.push(resolved);
                    pending_raw_indexes.remove(&i);
                    resolved_something = true;
                }
            }
            if !resolved_something {
                break;
            }
        }

        ensure!(
            pending_raw_indexes.is_empty(),
            "failed to resolve all packfile objects"
        );

        // verify checksum at data[offset..offset+20]
        const CHECKSUM_LEN: usize = 20;
        ensure!(
            data.len() >= offset + CHECKSUM_LEN,
            "truncated pack checksum"
        );
        let expected_checksum = &data[offset..offset + CHECKSUM_LEN];
        let actual_checksum = Sha1::digest(&data[..offset]);
        ensure!(
            expected_checksum == &actual_checksum[..],
            "invalid pack checksum: expected {}, got {}",
            hex::encode(expected_checksum),
            hex::encode(actual_checksum)
        );

        Ok(Self {
            version,
            n_objects,
            objects: resolved_objects,
        })
    }
}
