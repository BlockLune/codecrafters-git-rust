use anyhow::anyhow;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum RawKind {
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    OfsDelta = 6,
    RefDelta = 7,
}

impl TryFrom<u8> for RawKind {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> std::prelude::v1::Result<Self, Self::Error> {
        match value {
            1 => Ok(RawKind::Commit),
            2 => Ok(RawKind::Tree),
            3 => Ok(RawKind::Blob),
            4 => Ok(RawKind::Tag),
            6 => Ok(RawKind::OfsDelta),
            7 => Ok(RawKind::RefDelta),
            _ => Err(anyhow!("invalid raw object kind: {}", value)),
        }
    }
}

#[derive(Debug)]
pub enum BaseKind {
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
}

impl TryFrom<u8> for BaseKind {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> std::prelude::v1::Result<Self, Self::Error> {
        match value {
            1 => Ok(BaseKind::Commit),
            2 => Ok(BaseKind::Tree),
            3 => Ok(BaseKind::Blob),
            4 => Ok(BaseKind::Tag),
            _ => Err(anyhow!("invalid base object kind: {}", value)),
        }
    }
}

impl TryFrom<RawKind> for BaseKind {
    type Error = anyhow::Error;

    fn try_from(value: RawKind) -> Result<Self, Self::Error> {
        match value {
            RawKind::Commit => Ok(BaseKind::Commit),
            RawKind::Tree => Ok(BaseKind::Tree),
            RawKind::Blob => Ok(BaseKind::Blob),
            RawKind::Tag => Ok(BaseKind::Tag),
            _ => Err(anyhow!("can't convert")),
        }
    }
}
