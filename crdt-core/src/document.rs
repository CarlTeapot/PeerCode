use crate::store::{StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};

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

    pub fn get_text(&self) -> String {
        let mut text = String::new();
        let mut curr = self.head;

        while let Some(id) = curr {
            if let Some(block) = self.store.get(&id) {
                if !block.is_deleted {
                    text.push_str(block.content());
                }
                curr = block.right();
            } else {
                break;
            }
        }
        text
    }

    pub fn insert(&mut self, position: u64, content: &str) {
        if content.is_empty() {
            return;
        }

        let mut left_neighbor: Option<BlockId> = None;
        let mut right_neighbor: Option<BlockId>;

        let (block, offset) = self.get_block_and_offset_by_position(position);
        if let Some(block_id) = block {
            if offset == 0 {
                let block = self.store.get(&block_id).unwrap();
                left_neighbor = block.left();
                right_neighbor = Some(block_id);
            } else {
                self.split_block(block_id, offset);
                left_neighbor = Some(block_id);
                right_neighbor = self.store.get(&block_id).unwrap().right();
            }
        } else {
            let mut curr = self.head;
            while let Some(id) = curr {
                left_neighbor = Some(id);
                curr = self.store.get(&id).unwrap().right();
            }
            right_neighbor = None;
        }

        let next_clock = self.state_vector.get(&self.client_id);
        let new_id = BlockId::new(self.client_id, Clock::new(next_clock));

        let new_block = Block::new(new_id, left_neighbor, right_neighbor, content.to_string());

        let block_len = new_block.len;
        self.store.insert(new_block);

        if let Some(left_id) = left_neighbor {
            self.store
                .get_mut(&left_id)
                .unwrap()
                .set_right(Some(new_id));
        } else {
            self.head = Some(new_id);
        }

        if let Some(right_id) = right_neighbor {
            self.store
                .get_mut(&right_id)
                .unwrap()
                .set_left(Some(new_id));
        }

        self.state_vector
            .update(self.client_id, next_clock + block_len);
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
