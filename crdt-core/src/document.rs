use crate::store::{StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId};

#[derive(Debug)]
pub struct Document {
    pub client_id: ClientId,
    pub store: StructStore,
    pub state_vector: StateVector,
    pub head: Option<BlockId>,
}

impl Document {
    pub fn new(client_id: ClientId) -> Self {
        Document {
            client_id,
            store: StructStore::new(),
            state_vector: StateVector::new(),
            head: None,
        }
    }

    pub fn insert(&mut self, _position: u64, _content: &str) {
        // giorgi gelashvili, dabadebuli 2004 wels
    }

    pub fn delete(&mut self, _position: u64, _length: u64) {
        // chichikia
    }
    //remove the annotation after using this function
    #[allow(dead_code)]
    fn split_block(&mut self, block_id: BlockId, offset: u64) {
        let (right_block_id, new_block) = {
            let block = self.store.get_mut(&block_id).unwrap();

            if offset == 0 || offset >= block.len {
                return; // no need to split
            };

            let new_block_content: String = block.content().chars().skip(offset as usize).collect();
            block.set_content(block.content().chars().take(offset as usize).collect());

            let new_block_id = BlockId {
                client: block.id.client,
                clock: block.id.clock.advance(offset),
            };
            let mut new_block: Block = Block::new(
                new_block_id,
                Some(block.id),
                block.right(),
                new_block_content,
            );

            new_block.is_deleted = block.is_deleted;

            let old_right_block_id = block.right();
            block.set_right(Some(new_block_id));
            block.origin_right = Some(new_block_id);

            (old_right_block_id, new_block)
        };

        let new_block_id = new_block.id;
        self.store.insert_after_block(&block_id, new_block);

        if right_block_id.is_none() {
            return;
        }

        self.store
            .get_mut(&right_block_id.unwrap())
            .unwrap()
            .set_left(Some(new_block_id));
    }

    // use this to get the block by position in the text editor
    // offset variable is used for splitting
    // washalet es komentarebi ro morchebit da anotaciac
    #[allow(dead_code)]
    fn get_block_and_offset_by_position(&self, mut position: u64) -> (Option<BlockId>, u64) {
        let mut current_block = self.head.and_then(|id| self.store.get(&id));

        while let Some(block) = current_block {
            if block.is_deleted {
                current_block = block.right().and_then(|id| self.store.get(&id));
                continue;
            }

            let content_len = block.len;

            if position < content_len {
                return (Some(block.id), position);
            }
            position -= content_len;
            current_block = block.right().and_then(|id| self.store.get(&id));
        }

        (None, position)
    }
}

#[cfg(test)]
mod tests;
