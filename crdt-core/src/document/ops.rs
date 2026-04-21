use super::Document;
use crate::error::DocumentError;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, Clock};

impl Document {
    fn resolve_origins(
        &mut self,
        position: u64,
    ) -> Result<(Option<BlockId>, Option<BlockId>), DocumentError> {
        let (block, offset, tail_id) = self.get_block_and_offset_by_position(position);

        let Some(block_id) = block else {
            if offset > 0 {
                return Err(DocumentError::OutOfBounds(position));
            }
            return Ok((tail_id, None));
        };

        if offset == 0 {
            let block_ref = self
                .store
                .get(&block_id)
                .ok_or(DocumentError::BlockNotFound(block_id))?;
            return Ok((block_ref.left(), Some(block_id)));
        }

        self.split_block(block_id, offset)?;
        let left_ref = self
            .store
            .get(&block_id)
            .ok_or(DocumentError::BlockNotFound(block_id))?;
        let origin_left_id = BlockId::new(block_id.client, block_id.clock.advance(offset - 1));
        Ok((Some(origin_left_id), left_ref.right()))
    }

    pub fn local_insert(&mut self, position: u64, content: &str) -> Result<(), DocumentError> {
        if content.is_empty() {
            return Ok(());
        }

        let (left_origin, right_origin) = self.resolve_origins(position)?;

        let next_clock = self.state_vector.get(&self.client_id);
        let new_id = BlockId::new(self.client_id, Clock::new(next_clock));
        let new_block = Block::new(new_id, left_origin, right_origin, content.to_string());
        let block_len = new_block.len;

        self.integrate(new_block)?;
        self.state_vector
            .update(self.client_id, next_clock + block_len);

        Ok(())
    }

    /// Delete `length` visible characters starting at `position`.
    pub fn delete(&mut self, position: u64, length: u64) -> Result<(), DocumentError> {
        if length == 0 {
            return Ok(());
        }

        let (first_id, start_offset, _) = self.get_block_and_offset_by_position(position);

        let Some(mut current_id) = first_id else {
            return Err(DocumentError::OutOfBounds(position));
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

        if remaining > 0 {
            return Err(DocumentError::OutOfBounds(position + length - remaining));
        }

        Ok(())
    }

    /// Reclaim storage for every block whose tombstone is covered by
    /// `confirmed`. Content bytes are cleared
    pub fn collect_garbage(&mut self, confirmed: &DeleteSet) {
        for (client, range) in confirmed.iter() {
            let mut current_clock = range.start;
            let end_clock = range.end();

            while current_clock < end_clock {
                let id = BlockId::new(*client, Clock::new(current_clock));

                let Some(block) = self.store.get(&id) else {
                    break;
                };
                let next_clock = block.id.clock.value + block.len;

                self.store.erase_content(&id);
                current_clock = next_clock;
            }
        }
    }
}
