use std::collections::HashMap;

use anyhow::{Context, Result, ensure};
use sha1::{Digest, Sha1};

mod delta;
mod kind;

use crate::util::compression::decompress_zlib;
use delta::{Delta, apply_delta, parse_delta};
use kind::{BaseKind, RawKind};

#[derive(Debug)]
#[allow(unused)]
pub enum RawPackObj {
    Base {
        offset: usize,
        kind: BaseKind,
        size: usize,
        data: Vec<u8>,
    },
    OfsDelta {
        offset: usize,
        size: usize,
        base_distance: usize,
        delta: Delta,
    },
    RefDelta {
        offset: usize,
        size: usize,
        base_sha1: [u8; 20],
        delta: Delta,
    },
}

impl RawPackObj {
    pub fn offset(&self) -> usize {
        match self {
            RawPackObj::Base {
                offset,
                kind: _,
                size: _,
                data: _,
            } => *offset,
            RawPackObj::OfsDelta {
                offset,
                size: _,
                base_distance: _,
                delta: _,
            } => *offset,
            RawPackObj::RefDelta {
                offset,
                size: _,
                base_sha1: _,
                delta: _,
            } => *offset,
        }
    }

    pub fn is_ref_delta(&self) -> bool {
        matches!(
            self,
            Self::RefDelta {
                offset: _,
                size: _,
                base_sha1: _,
                delta: _
            }
        )
    }
}

pub fn parse_next_raw_pack_obj(data: &[u8], pack_offset: usize) -> Result<(RawPackObj, usize)> {
    ensure!(!data.is_empty(), "truncated pack object");

    let first_byte = data[0];
    let raw_kind = RawKind::try_from((first_byte >> 4) & 0b111)?;

    // MSB -> calc size field
    let mut size = (first_byte & 0b1111) as usize;
    let mut idx = 1;
    let mut shift = 4;
    while idx < data.len() && (data[idx - 1] & 0b1000_0000) != 0 {
        size |= ((data[idx] & 0b0111_1111) as usize) << shift;
        shift += 7;
        idx += 1;
    }

    let header_len = idx;

    match raw_kind {
        RawKind::Commit | RawKind::Tree | RawKind::Blob | RawKind::Tag => {
            let base_kind = BaseKind::try_from(raw_kind)?;
            let (decompressed, compressed_len) = decompress_zlib(&data[header_len..])?;

            let obj = RawPackObj::Base {
                offset: pack_offset,
                kind: base_kind,
                size,
                data: decompressed,
            };
            let consumed = header_len + compressed_len;

            Ok((obj, consumed))
        }
        RawKind::OfsDelta => {
            let (base_distance, base_ref_len) = parse_ofs_delta(&data[header_len..])?;
            let (delta_data, compressed_len) = decompress_zlib(&data[header_len + base_ref_len..])?;
            let delta = parse_delta(&delta_data)?;

            let obj = RawPackObj::OfsDelta {
                offset: pack_offset,
                size,
                base_distance,
                delta,
            };
            let consumed = header_len + base_ref_len + compressed_len;

            Ok((obj, consumed))
        }
        RawKind::RefDelta => {
            let (base_sha1, base_ref_len) = parse_ref_delta(&data[header_len..])?;
            let (delta_data, compressed_len) = decompress_zlib(&data[header_len + base_ref_len..])?;
            let delta = parse_delta(&delta_data)?;

            let obj = RawPackObj::RefDelta {
                offset: pack_offset,
                size,
                base_sha1,
                delta,
            };
            let consumed = header_len + base_ref_len + compressed_len;

            Ok((obj, consumed))
        }
    }
}

fn parse_ofs_delta(data: &[u8]) -> Result<(usize, usize)> {
    ensure!(!data.is_empty(), "truncated ofs-delta offset");

    let mut offset: usize = (data[0] & 0b01111111) as usize;
    let mut i: usize = 1;
    while i < data.len() && (data[i - 1] & 0b10000000) != 0 {
        offset = (offset + 1) << 7 | (data[i] & 0b01111111) as usize;
        i += 1;
    }
    ensure!(data[i - 1] & 0b10000000 == 0, "truncated ofs-delta offset");

    Ok((offset, i))
}

fn parse_ref_delta(data: &[u8]) -> Result<([u8; 20], usize)> {
    let base_sha1: [u8; 20] = data[..20].try_into().context("truncated ref-delta")?;
    Ok((base_sha1, 20))
}

#[derive(Clone)]
pub struct ResolvedPackObj {
    offset: usize,
    kind: BaseKind,
    data: Vec<u8>,
}

impl ResolvedPackObj {
    pub fn sha1(&self) -> [u8; 20] {
        let kind_name = self.kind.to_string();
        let mut data = Vec::from(format!("{} {}\0", kind_name, self.data.len()).as_bytes());
        data.extend_from_slice(&self.data);
        Sha1::digest(data).try_into().unwrap()
    }

    pub fn try_from_raw(
        index: usize,
        raw_objects: &[RawPackObj],
        offset_to_raw_index: &HashMap<usize, usize>,
        sha1_to_raw_index: &mut HashMap<[u8; 20], usize>,
        resolved_cache: &mut HashMap<usize, ResolvedPackObj>,
    ) -> Result<Option<Self>> {
        if let Some(resolved) = resolved_cache.get(&index) {
            return Ok(Some(resolved.clone()));
        }

        let resolved = match &raw_objects[index] {
            RawPackObj::Base {
                offset,
                kind,
                size: _,
                data,
            } => Some(ResolvedPackObj {
                offset: *offset,
                kind: kind.clone(),
                data: data.to_vec(),
            }),
            RawPackObj::OfsDelta {
                offset,
                size: _,
                base_distance,
                delta,
            } => {
                let base_offset = offset - base_distance;
                let base_index = *offset_to_raw_index.get(&base_offset).context(format!(
                    "failed to get raw object's index from offset {}",
                    offset
                ))?;

                match Self::try_from_raw(
                    base_index,
                    raw_objects,
                    offset_to_raw_index,
                    sha1_to_raw_index,
                    resolved_cache,
                )? {
                    Some(base_resolved) => Some(ResolvedPackObj {
                        offset: *offset,
                        kind: base_resolved.kind,
                        data: apply_delta(&base_resolved.data, delta)?,
                    }),
                    None => None,
                }
            }
            RawPackObj::RefDelta {
                offset,
                size: _,
                base_sha1,
                delta,
            } => match sha1_to_raw_index.get(base_sha1) {
                Some(&base_index) => {
                    match Self::try_from_raw(
                        base_index,
                        raw_objects,
                        offset_to_raw_index,
                        sha1_to_raw_index,
                        resolved_cache,
                    )? {
                        Some(base_resolved) => Some(ResolvedPackObj {
                            offset: *offset,
                            kind: base_resolved.kind.clone(),
                            data: apply_delta(&base_resolved.data, delta)?,
                        }),
                        None => None,
                    }
                }
                None => None,
            },
        };

        if let Some(resolved) = resolved.clone() {
            // register this resolved object's sha for later ref-delta lookups
            sha1_to_raw_index.insert((&resolved).sha1(), index);
            resolved_cache.insert(index, resolved);
        }

        Ok(resolved)
    }
}
