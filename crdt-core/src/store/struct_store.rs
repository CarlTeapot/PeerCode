use crate::store::StateVector;
use crate::structs::Block;

use crate::types::{BlockId, ClientId};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct StructStore {
    blocks: HashMap<ClientId, Vec<Block>>,
}

impl StructStore {
    pub fn new() -> Self {
        StructStore::default()
    }

    pub fn contains_key(&self, id: &BlockId) -> bool {
        self.get(id).is_some()
    }

    pub fn insert(&mut self, block: Block) {
        let list = self.blocks.entry(block.id.client).or_default();
        list.push(block);
    }

    pub fn insert_after_block(&mut self, prev_block_id: &BlockId, block: Block) {
        let client = prev_block_id.client;
        let list = self
            .blocks
            .get_mut(&client)
            .expect("client list must exist");
        let idx = Self::find_index(list, prev_block_id.clock.value).expect("prev block must exist");
        list.insert(idx + 1, block);
    }

    pub fn get(&self, id: &BlockId) -> Option<&Block> {
        let list = self.blocks.get(&id.client)?;
        let idx = Self::find_index(list, id.clock.value)?;
        Some(&list[idx])
    }

    pub fn get_mut(&mut self, id: &BlockId) -> Option<&mut Block> {
        let list = self.blocks.get_mut(&id.client)?;
        let idx = Self::find_index(list, id.clock.value)?;
        Some(&mut list[idx])
    }

    pub fn state_vector(&self) -> StateVector {
        let mut sv = StateVector::new();
        for (client, blocks) in &self.blocks {
            if let Some(last) = blocks.last() {
                sv.update(*client, last.id.clock.value + last.len);
            }
        }
        sv
    }

    pub fn mark_deleted(&mut self, id: &BlockId) -> Option<&mut Block> {
        let block = self.get_mut(id)?;
        if !block.is_deleted {
            block.is_deleted = true;
        }
        Some(block)
    }

    pub fn erase_content(&mut self, id: &BlockId) -> bool {
        match self.get_mut(id) {
            Some(block) if block.is_deleted && !block.is_empty() => {
                block.clear_content_for_gc();
                true
            }
            _ => false,
        }
    }

    fn find_index(list: &[Block], clock: u64) -> Option<usize> {
        let result = list.partition_point(|b| b.id.clock.value <= clock);
        if result == 0 {
            return None;
        }
        let idx = result - 1;
        let b = &list[idx];
        if clock < b.id.clock.value + b.len {
            Some(idx)
        } else {
            None
        }
    }

    // Empty for now , will implement later when we have the basic structure in place
    //pub fn get_missing_blocks(&self, remote_sv: &StateVector) -> Vec<&Block> {    }

    //pub fn split_block_at(&mut self, client: ClientId, split_clock: u64) -> Option<BlockId> {}

    //pub fn try_squash_tail(&mut self, client: ClientId) {}

    //pub fn undelete(&mut self, id: &BlockId) -> bool {}
}
