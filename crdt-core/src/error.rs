use crate::types::BlockId;

#[derive(Debug)]
pub enum DocumentError {
    BlockNotFound(BlockId),
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentError::BlockNotFound(id) => {
                write!(
                    f,
                    "block not found: client={}, clock={}",
                    id.client.value, id.clock.value
                )
            }
        }
    }
}

impl std::error::Error for DocumentError {}
