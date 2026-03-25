use crate::store::{StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock, DocumentError};

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

    fn integrate(&mut self, block: Block) -> Result<BlockId, DocumentError> {
        let block_id = block.id;

        let mut left = block.origin_left;
        let right = block.origin_right;

        let mut scanning_id = if let Some(id) = left {
            self.store
                .get(&id)
                .ok_or(DocumentError::BlockNotFound(id))?
                .right()
        } else {
            self.head
        };

        while let Some(curr_id) = scanning_id {
            if Some(curr_id) == right {
                break;
            }

            let curr_block = self
                .store
                .get(&curr_id)
                .ok_or(DocumentError::BlockNotFound(curr_id))?;

            let o_l = curr_block.origin_left;
            let o_r = curr_block.origin_right;

            let mut conflict = false;
            if o_l == block.origin_left {
                if o_r == block.origin_right && block.id.client.value < curr_block.id.client.value {
                    break;
                }
                conflict = true;
            }

            if !conflict {
                break;
            }

            left = Some(curr_id);
            scanning_id = curr_block.right();
        }

        let final_left = left;
        let final_right = if let Some(l_id) = final_left {
            self.store
                .get(&l_id)
                .ok_or(DocumentError::BlockNotFound(l_id))?
                .right()
        } else {
            self.head
        };

        if let Some(id) = final_left
            && !self.store.contains_key(&id)
        {
            return Err(DocumentError::BlockNotFound(id));
        }
        if let Some(id) = final_right
            && !self.store.contains_key(&id)
        {
            return Err(DocumentError::BlockNotFound(id));
        }

        self.store.insert(block);

        if let Some(l_id) = final_left {
            self.store
                .get_mut(&l_id)
                .ok_or(DocumentError::BlockNotFound(l_id))?
                .set_right(Some(block_id));
        } else {
            self.head = Some(block_id);
        }

        if let Some(r_id) = final_right {
            self.store
                .get_mut(&r_id)
                .ok_or(DocumentError::BlockNotFound(r_id))?
                .set_left(Some(block_id));
        }

        let b_mut = self
            .store
            .get_mut(&block_id)
            .ok_or(DocumentError::BlockNotFound(block_id))?;
        b_mut.set_left(final_left);
        b_mut.set_right(final_right);

        Ok(block_id)
    }

    pub fn local_insert(&mut self, position: u64, content: &str) -> Result<(), DocumentError> {
        if content.is_empty() {
            return Ok(());
        }

        let (block, offset, tail_id) = self.get_block_and_offset_by_position(position);

        let (left_origin, right_origin) = if let Some(block_id) = block {
            if offset == 0 {
                let block_ref = self
                    .store
                    .get(&block_id)
                    .ok_or(DocumentError::BlockNotFound(block_id))?;
                (block_ref.left(), Some(block_id))
            } else {
                self.split_block(block_id, offset);
                let left_ref = self
                    .store
                    .get(&block_id)
                    .ok_or(DocumentError::BlockNotFound(block_id))?;
                (Some(block_id), left_ref.right())
            }
        } else {
            if offset > 0 && self.head.is_some() {
                return Err(DocumentError::OutOfBounds(position));
            }
            (tail_id, None)
        };

        let next_clock = self.state_vector.get(&self.client_id);
        let new_id = BlockId::new(self.client_id, Clock::new(next_clock));

        let new_block = Block::new(new_id, left_origin, right_origin, content.to_string());

        let block_len = new_block.len;

        self.integrate(new_block)?;
        self.state_vector
            .update(self.client_id, next_clock + block_len);

        Ok(())
    }

    pub fn remote_insert(&mut self, block: Block) -> Result<(), DocumentError> {
        let client = block.id.client;
        let end_clock = block.id.clock.value + block.len;

        self.integrate(block)?;
        self.state_vector.update(client, end_clock);
        Ok(())
    }

    pub fn delete(&mut self, _position: u64, _length: u64) {
        // chichikia
    }

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

    fn get_block_and_offset_by_position(
        &self,
        mut position: u64,
    ) -> (Option<BlockId>, u64, Option<BlockId>) {
        let mut current_block = self.head.and_then(|id| self.store.get(&id));
        let mut tail_id = None;

        while let Some(block) = current_block {
            tail_id = Some(block.id);

            if block.is_deleted {
                current_block = block.right().and_then(|id| self.store.get(&id));
                continue;
            }

            let content_len = block.len;

            if position < content_len {
                return (Some(block.id), position, tail_id);
            }
            position -= content_len;
            current_block = block.right().and_then(|id| self.store.get(&id));
        }

        (None, position, tail_id)
    }
}

#[cfg(test)]
mod tests;
