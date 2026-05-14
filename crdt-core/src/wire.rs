use crate::snapshot::{Snapshot, SnapshotError};
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::BlockId;
use log::trace;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

pub const OP_PREFIX: u8 = 0x00;
pub const SNAPSHOT_PREFIX: u8 = 0x01;
pub const PREFIX_CONTROL: u8 = 0x02;
pub const CONTROL_SESSION_ENDED: u8 = 0x01;
pub const CONTROL_ROOM_STATE: u8 = 0x02;
pub const CONTROL_PERMISSION_CHANGE: u8 = 0x03;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub client_id: String,
    pub username: String,
    pub is_host: bool,
    pub can_write: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomState {
    pub peers: Vec<PeerInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionChange {
    pub target_client_id: String,
    pub can_write: bool,
}

#[derive(Debug)]
pub enum WireError {
    EmptyFrame,
    UnknownPrefix(u8),
    NotAnOp,
    NotASnapshot,
    NotAControl,
    ControlSubTypeMismatch { expected: u8, got: u8 },
    Decode(bitcode::Error),
    SnapshotDecode(SnapshotError),
    JsonDecode(String),
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
            WireError::NotASnapshot => {
                write!(f, "wire frame carries an op, not a snapshot")
            }
            WireError::NotAControl => {
                write!(f, "wire frame is not a control frame")
            }
            WireError::ControlSubTypeMismatch { expected, got } => {
                write!(
                    f,
                    "control sub-type mismatch: expected 0x{expected:02X}, got 0x{got:02X}"
                )
            }
            WireError::Decode(e) => write!(f, "bitcode decode failed: {e}"),
            WireError::SnapshotDecode(e) => write!(f, "snapshot decode failed: {e}"),
            WireError::JsonDecode(e) => write!(f, "json decode failed: {e}"),
        }
    }
}

impl Error for WireError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WireError::Decode(e) => Some(e),
            WireError::SnapshotDecode(e) => Some(e),
            _ => None,
        }
    }
}

pub fn encode_op(msg: &OpMessage) -> Vec<u8> {
    trace!("encode operation requested: {:?}", msg);
    let payload = bitcode::encode(msg);
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(OP_PREFIX);
    frame.extend_from_slice(&payload);
    trace!("encode frame encoded: {frame:?}");
    frame
}

pub fn decode_op(frame: &[u8]) -> Result<OpMessage, WireError> {
    trace!("decode operation requested: {:?}", frame);
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    match prefix {
        OP_PREFIX => bitcode::decode(payload).map_err(WireError::Decode),
        SNAPSHOT_PREFIX => Err(WireError::NotAnOp),
        b => Err(WireError::UnknownPrefix(b)),
    }
}

pub fn encode_snapshot(snap: &Snapshot) -> Vec<u8> {
    trace!("encode snapshot requested");
    let payload = snap.encode();
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(SNAPSHOT_PREFIX);
    frame.extend_from_slice(&payload);
    trace!("encode snapshot frame: {} bytes", frame.len());
    frame
}

pub fn decode_snapshot(frame: &[u8]) -> Result<Snapshot, WireError> {
    trace!("decode snapshot requested: {} bytes", frame.len());
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    match prefix {
        SNAPSHOT_PREFIX => Snapshot::decode(payload).map_err(WireError::SnapshotDecode),
        OP_PREFIX => Err(WireError::NotASnapshot),
        b => Err(WireError::UnknownPrefix(b)),
    }
}

pub fn encode_control_json<T: Serialize>(sub_type: u8, payload: &T) -> Vec<u8> {
    let json = serde_json::to_vec(payload).expect("control payload serialization cannot fail");
    let mut frame = Vec::with_capacity(2 + json.len());
    frame.push(PREFIX_CONTROL);
    frame.push(sub_type);
    frame.extend_from_slice(&json);
    frame
}

pub fn decode_control_json<T: for<'de> Deserialize<'de>>(
    frame: &[u8],
    expected_sub_type: u8,
) -> Result<T, WireError> {
    if frame.len() < 2 {
        return Err(WireError::EmptyFrame);
    }
    if frame[0] != PREFIX_CONTROL {
        return Err(WireError::NotAControl);
    }
    if frame[1] != expected_sub_type {
        return Err(WireError::ControlSubTypeMismatch {
            expected: expected_sub_type,
            got: frame[1],
        });
    }
    serde_json::from_slice(&frame[2..]).map_err(|e| WireError::JsonDecode(e.to_string()))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod protocol_drift_tests;
