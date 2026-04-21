use crate::store::{DeleteSet, StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId};

mod integrate;
mod ops;
mod pending;
mod traversal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteChange {
    Insert { position: u64, content: String },
    Delete { position: u64, length: u64 },
}

#[derive(Debug)]
pub struct Document {
    pub client_id: ClientId,
    pub store: StructStore,
    pub state_vector: StateVector,
    pub delete_set: DeleteSet,
    pub seen_delete_set: DeleteSet,
    pub head: Option<BlockId>,
    pending_blocks: Vec<Block>,
    pending_delete_sets: Vec<DeleteSet>,
}

impl Document {
    pub fn new(client_id: ClientId) -> Self {
        Document {
            client_id,
            store: StructStore::new(),
            state_vector: StateVector::new(),
            delete_set: DeleteSet::new(),
            seen_delete_set: DeleteSet::new(),
            head: None,
            pending_blocks: Vec::new(),
            pending_delete_sets: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
