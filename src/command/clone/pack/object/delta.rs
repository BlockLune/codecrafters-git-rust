use anyhow::{Result, ensure};

#[derive(Debug)]
pub struct Delta {
    base_size: usize,
    result_size: usize,
    instructions: Vec<DeltaInstruction>,
}

#[derive(Debug)]
pub enum DeltaInstruction {
    Copy { offset: usize, size: usize },
    Insert(Vec<u8>),
}

pub fn parse_delta(data: &[u8]) -> Result<Delta> {
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
