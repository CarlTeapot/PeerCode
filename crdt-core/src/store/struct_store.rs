use crate::store::StateVector;
use crate::structs::Block;

use crate::types::{BlockId, ClientId};
use std::collections::HashMap;

// Strucstore: primary storage of blocks, indexed by client and clock.
#[derive(Debug, Default)]
pub struct StructStore {
    blocks: HashMap<ClientId, Vec<Block>>,
    index: HashMap<BlockId, usize>,
}

impl StructStore {
    pub fn new() -> Self {
        StructStore::default()
    }

    pub fn insert(&mut self, block: Block) {
        let client = block.id.client;
        let block_id = block.id; 
        let list = self.blocks.entry(client).or_default();
        let idx = list.len();
        list.push(block);
        
        self.index.insert(block_id, idx);
    }
    
    pub fn get(&self, id: &BlockId) -> Option<&Block> {
        let idx = self.index.get(id)?;
        let list = self.blocks.get(&id.client)?;
        list.get(*idx)
    }

    /// Look up a block mutably
    pub fn get_mut(&mut self, id: &BlockId) -> Option<&mut Block> {
        let idx = *self.index.get(id)?;
        let list = self.blocks.get_mut(&id.client)?;
        list.get_mut(idx)
    }

    /// Compute the current StateVector from the store.
    pub fn state_vector(&self) -> StateVector {
        let mut sv = StateVector::new();
        for (client, blocks) in &self.blocks {
            if let Some(last) = blocks.last() {
                sv.update(*client, last.id.clock.value + last.len());
            }
        }
        sv
    }

    // Empty for now , will implement later when we have the basic structure in place
    //pub fn get_missing_blocks(&self, remote_sv: &StateVector) -> Vec<&Block> {    }

    //pub fn split_block_at(&mut self, client: ClientId, split_clock: u64) -> Option<BlockId> {}

    //pub fn try_squash_tail(&mut self, client: ClientId) {}

    //pub fn mark_deleted(&mut self, id: &BlockId) -> bool {}

    //pub fn undelete(&mut self, id: &BlockId) -> bool {}
}
