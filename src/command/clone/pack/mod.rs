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

fn parse_delta_size(data: &[u8]) -> Result<(usize, usize)> {
    ensure!(!data.is_empty(), "truncated delta size");

    let mut size = 0usize;
    let mut shift = 0;
    let mut i = 0;

    loop {
        ensure!(i < data.len(), "truncated delta size");

        let byte = data[i];
        size |= ((byte & 0b01111111) as usize) << shift;
        i += 1;

        if byte & 0b10000000 == 0 {
            break;
        }

        shift += 7;
    }

    Ok((size, i))
}

#[derive(Debug)]
struct Delta {
    base_size: usize,
    result_size: usize,
    instructions: Vec<DeltaInstruction>,
}

#[derive(Debug)]
enum DeltaInstruction {
    Copy { offset: usize, size: usize },
    Insert(Vec<u8>),
}

fn parse_delta(data: &[u8]) -> Result<Delta> {
    let (base_size, base_size_len) = parse_delta_size(data)?;
    let (result_size, result_size_len) = parse_delta_size(&data[base_size_len..])?;

    let mut i = base_size_len + result_size_len;
    let mut instructions = Vec::new();

    while i < data.len() {
        let opcode = data[i];
        i += 1;

        if opcode & 0b10000000 != 0 {
            let mut offset = 0usize;
            let mut size = 0usize;

            // handle offset bits
            if opcode & 0b00000001 != 0 {
                offset |= data[i] as usize;
                i += 1;
            }
            if opcode & 0b00000010 != 0 {
                offset |= (data[i] as usize) << 8;
                i += 1;
            }
            if opcode & 0b00000100 != 0 {
                offset |= (data[i] as usize) << 16;
                i += 1;
            }
            if opcode & 0b00001000 != 0 {
                offset |= (data[i] as usize) << 24;
                i += 1;
            }

            // handle size bits
            if opcode & 0b00010000 != 0 {
                size |= data[i] as usize;
                i += 1;
            }
            if opcode & 0b00100000 != 0 {
                size |= (data[i] as usize) << 8;
                i += 1;
            }
            if opcode & 0b01000000 != 0 {
                size |= (data[i] as usize) << 16;
                i += 1;
            }

            // special rule
            if size == 0 {
                size = 0x10000;
            }

            instructions.push(DeltaInstruction::Copy { offset, size });
        } else {
            let size = opcode as usize;
            instructions.push(DeltaInstruction::Insert(data[i..i + size].to_vec()));
            i += size;
        }
    }

    Ok(Delta {
        base_size,
        result_size,
        instructions,
    })
}
