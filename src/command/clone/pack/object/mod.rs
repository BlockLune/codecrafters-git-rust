use anyhow::{Context, Result, ensure};

mod delta;
mod odata;
mod otype;

use crate::util::compression::decompress_zlib;
use odata::ObjectData;
use otype::ObjectType;

#[derive(Debug)]
pub struct PackFileObject {
    pub offset: usize,
    pub obj_type: ObjectType,
    pub obj_size: usize,
    pub data: ObjectData,
}

impl PackFileObject {
    pub fn parse_next(data: &[u8], pack_offset: usize) -> Result<(usize, Self)> {
        ensure!(!data.is_empty(), "truncated pack object");

        let first_byte = data[0];
        let obj_type = ObjectType::try_from((first_byte >> 4) & 0b111)?;

        // MSB
        let mut obj_size = (first_byte & 0b1111) as usize;
        let mut idx = 1;
        let mut shift = 4;
        while idx < data.len() && (data[idx - 1] & 0b10000000) != 0 {
            obj_size |= ((data[idx] & 0b01111111) as usize) << shift;
            shift += 7;
            idx += 1;
        }
        let header_len = idx;

        match obj_type {
            ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                let (decompressed, compressed_len) = decompress_zlib(&data[header_len..])?;
                Ok((
                    header_len + compressed_len,
                    Self {
                        offset: pack_offset,
                        obj_type,
                        obj_size,
                        data: ObjectData::Base(decompressed),
                    },
                ))
            }
            ObjectType::OfsDelta => {
                let (base_distance, base_ref_len) = parse_ofs_delta(&data[header_len..])?;
                let (delta_data, compressed_len) =
                    decompress_zlib(&data[header_len + base_ref_len..])?;
                Ok((
                    header_len + base_ref_len + compressed_len,
                    Self {
                        offset: pack_offset,
                        obj_type,
                        obj_size,
                        data: ObjectData::OfsDelta {
                            base_distance,
                            delta_data,
                        },
                    },
                ))
            }
            ObjectType::RefDelta => {
                let (base_sha1, base_ref_len) = parse_ref_delta(&data[header_len..])?;
                let (delta_data, compressed_len) =
                    decompress_zlib(&data[header_len + base_ref_len..])?;
                Ok((
                    header_len + base_ref_len + compressed_len,
                    Self {
                        offset: pack_offset,
                        obj_type,
                        obj_size,
                        data: ObjectData::RefDelta {
                            sha: base_sha1,
                            delta_data,
                        },
                    },
                ))
            }
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
