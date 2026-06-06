use anyhow::Result;
use bytes::Bytes;

const PKT_LINE_LEN_BYTES: usize = 4;

pub fn encode(payload: &str) -> String {
    let len = payload.as_bytes().len() + PKT_LINE_LEN_BYTES;
    format!("{:04x}{}", len, payload)
}

pub fn decode(data: Bytes) -> Result<Vec<Bytes>> {
    let mut payloads = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let length_hex_string = String::from_utf8_lossy(&data[i..i + PKT_LINE_LEN_BYTES]);
        let length = usize::from_str_radix(&length_hex_string, 16)?;
        if length == 0 {
            i += 4;
            continue;
        }
        let payload = Bytes::copy_from_slice(&data[i + PKT_LINE_LEN_BYTES..i + length]);
        payloads.push(payload);
        i += length;
    }
    Ok(payloads)
}
