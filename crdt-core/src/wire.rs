use std::fmt;

use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::BlockId;

pub const OP_PREFIX: u8 = 0x00;
pub const SNAPSHOT_PREFIX: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct WireBlock {
    pub id: BlockId,
    pub origin_left: Option<BlockId>,
    pub origin_right: Option<BlockId>,
    pub content: String,
}

impl From<&Block> for WireBlock {
    fn from(b: &Block) -> Self {
        WireBlock {
            id: b.id,
            origin_left: b.origin_left,
            origin_right: b.origin_right,
            content: b.content().to_string(),
        }
    }
}

impl From<WireBlock> for Block {
    fn from(w: WireBlock) -> Self {
        Block::new(w.id, w.origin_left, w.origin_right, w.content)
    }
}

#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub enum OpMessage {
    Insert(WireBlock),
    Delete(DeleteSet),
}

#[derive(Debug)]
pub enum WireError {
    EmptyFrame,
    UnknownPrefix(u8),
    NotAnOp,
    Decode(bitcode::Error),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::EmptyFrame => write!(f, "wire frame is empty"),
            WireError::UnknownPrefix(b) => {
                write!(f, "unknown wire prefix byte: 0x{b:02X}")
            }
            WireError::NotAnOp => {
                write!(f, "wire frame carries a snapshot, not an op")
            }
            WireError::Decode(e) => write!(f, "bitcode decode failed: {e}"),
        }
    }
}

impl std::error::Error for WireError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WireError::Decode(e) => Some(e),
            _ => None,
        }
    }
}

pub fn encode_op(msg: &OpMessage) -> Vec<u8> {
    let payload = bitcode::encode(msg);
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(OP_PREFIX);
    frame.extend_from_slice(&payload);
    frame
}

pub fn decode_op(frame: &[u8]) -> Result<OpMessage, WireError> {
    match frame.first() {
        None => Err(WireError::EmptyFrame),
        Some(&OP_PREFIX) => bitcode::decode(&frame[1..]).map_err(WireError::Decode),
        Some(&SNAPSHOT_PREFIX) => Err(WireError::NotAnOp),
        Some(&b) => Err(WireError::UnknownPrefix(b)),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod protocol_drift_tests;
