use crate::error::DocumentError;
use crate::store::{DeleteSet, StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};

#[derive(Debug)]
pub struct Document {
    pub client_id: ClientId,
    pub store: StructStore,
    pub state_vector: StateVector,
    pub delete_set: DeleteSet,
    pub seen_delete_set: DeleteSet,
    pub head: Option<BlockId>,
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
        }
    }

    pub fn insert(&mut self, _position: u64, _content: &str) {
        // giorgi gelashvili, dabadebuli 2004 wels
    }

    pub fn delete(&mut self, position: u64, length: u64) -> Result<(), DocumentError> {
        if length == 0 {
            return Ok(());
        }

        let (first_id, start_offset) = self.get_block_and_offset_by_position(position);

        let Some(mut current_id) = first_id else {
            return Ok(());
        };

        if start_offset > 0
            && let Some(new_id) = self.split_block(current_id, start_offset)?
        {
            current_id = new_id;
        }

        let mut remaining = length;

        while remaining > 0 {
            let (is_deleted, block_len, right_id) = {
                let block = match self.store.get(&current_id) {
                    Some(b) => b,
                    None => break,
                };
                (block.is_deleted, block.len, block.right())
            };

            if is_deleted {
                match right_id {
                    Some(next) => {
                        current_id = next;
                        continue;
                    }
                    None => break,
                }
            }

            if block_len > remaining {
                self.split_block(current_id, remaining)?;
            }

            let (deleted_len, next_id) = {
                let block = self
                    .store
                    .mark_deleted(&current_id)
                    .ok_or(DocumentError::BlockNotFound(current_id))?;
                (block.len, block.right())
            };

            self.delete_set.add(current_id, deleted_len);
            remaining = remaining.saturating_sub(deleted_len);

            match next_id {
                Some(next) => current_id = next,
                None => break,
            }
        }

        Ok(())
    }

    pub fn apply_delete_set(&mut self, remote: &DeleteSet) -> Result<(), DocumentError> {
        for (client, range) in remote.iter() {
            let mut current_clock = range.start;
            let end_clock = range.end();

            while current_clock < end_clock {
                let id = BlockId::new(*client, Clock::new(current_clock));

                let (block_start, block_len, block_id) = match self.store.get(&id) {
                    Some(b) => (b.id.clock.value, b.len, b.id),
                    None => {
                        current_clock += 1;
                        continue;
                    }
                };

                let offset = current_clock - block_start;
                if offset > 0 {
                    self.split_block(block_id, offset)?;
                    continue;
                }

                let remaining_delete = end_clock - current_clock;
                if block_len > remaining_delete {
                    self.split_block(block_id, remaining_delete)?;
                }

                let actual_len = self
                    .store
                    .mark_deleted(&block_id)
                    .ok_or(DocumentError::BlockNotFound(block_id))?
                    .len;

                current_clock += actual_len;
            }
        }

        self.seen_delete_set.merge(remote);

        Ok(())
    }

    pub fn collect_garbage(&mut self, confirmed: &DeleteSet) {
        for (client, range) in confirmed.iter() {
            let mut current_clock = range.start;
            let end_clock = range.end();

            while current_clock < end_clock {
                let id = BlockId::new(*client, Clock::new(current_clock));

                let next_clock = match self.store.get(&id) {
                    Some(b) => b.id.clock.value + b.len,
                    None => current_clock + 1,
                };

                self.store.erase_content(&id);
                current_clock = next_clock;
            }
        }
    }

    fn split_block(
        &mut self,
        block_id: BlockId,
        offset: u64,
    ) -> Result<Option<BlockId>, DocumentError> {
        let (right_block_id, new_block) = {
            let block = self
                .store
                .get_mut(&block_id)
                .ok_or(DocumentError::BlockNotFound(block_id))?;

            if offset == 0 || offset >= block.len {
                return Ok(None);
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

        if let Some(right_id) = right_block_id {
            self.store
                .get_mut(&right_id)
                .ok_or(DocumentError::BlockNotFound(right_id))?
                .set_left(Some(new_block_id));
        }

        Ok(Some(new_block_id))
    }

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
