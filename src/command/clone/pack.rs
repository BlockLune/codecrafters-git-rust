use anyhow::{Context, Result, anyhow, ensure};
use std::convert::TryFrom;

use crate::util::compression::decompress_zlib;

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

#[derive(Debug)]
pub enum ObjectType {
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    OfsDelta = 6,
    RefDelta = 7,
}

impl TryFrom<u8> for ObjectType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> std::prelude::v1::Result<Self, Self::Error> {
        match value {
            1 => Ok(ObjectType::Commit),
            2 => Ok(ObjectType::Tree),
            3 => Ok(ObjectType::Blob),
            4 => Ok(ObjectType::Tag),
            6 => Ok(ObjectType::OfsDelta),
            7 => Ok(ObjectType::RefDelta),
            _ => Err(anyhow!("invalid object type: {}", value)),
        }
    }
}

#[derive(Debug)]
pub enum ObjectData {
    Base(Vec<u8>),
    OfsDelta {
        base_distance: usize,
        delta_data: Vec<u8>,
    },
    RefDelta { sha: [u8; 20], delta_data: Vec<u8> },
}

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
    ensure!(
        data[i - 1] & 0b10000000 == 0,
        "truncated ofs-delta offset"
    );

    Ok((offset, i))
}

fn parse_ref_delta(data: &[u8]) -> Result<([u8; 20], usize)> {
    let base_sha1: [u8; 20] = data[..20].try_into().context("truncated ref-delta")?;
    Ok((base_sha1, 20))
}
