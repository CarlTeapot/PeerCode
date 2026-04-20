use crate::types::BlockId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    BlockNotFound(BlockId),
    OutOfBounds(u64),
    PendingQueueFull,
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentError::BlockNotFound(id) => write!(
                f,
                "block not found: client={}, clock={}",
                id.client.value, id.clock.value
            ),
            DocumentError::OutOfBounds(pos) => {
                write!(f, "position {pos} is out of bounds")
            }
            DocumentError::PendingQueueFull => {
                write!(f, "pending queue exceeded its bound")
            }
        }
    }
}

impl std::error::Error for DocumentError {}
