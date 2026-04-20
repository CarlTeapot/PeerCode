use crate::error::DocumentError;
use crate::store::{DeleteSet, StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readiness {
    Ready,
    Pending,
    Duplicate,
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

    pub fn get_text(&self) -> String {
        let mut text = String::new();
        let mut curr = self.head;

        #[cfg(debug_assertions)]
        let max_steps = self.store.total_blocks().saturating_add(1);
        #[cfg(debug_assertions)]
        let mut steps: usize = 0;

        while let Some(id) = curr {
            #[cfg(debug_assertions)]
            {
                steps += 1;
                debug_assert!(
                    steps <= max_steps,
                    "cycle detected in document linked list at block {id:?}"
                );
            }

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

    #[cfg(debug_assertions)]
    pub fn debug_linked_list(&self) -> String {
        let mut parts = Vec::new();
        let mut curr = self.head;
        let max_steps = self.store.total_blocks().saturating_add(1);
        let mut steps: usize = 0;

        while let Some(id) = curr {
            steps += 1;
            debug_assert!(
                steps <= max_steps,
                "cycle detected in document linked list at block {id:?}"
            );

            if let Some(block) = self.store.get(&id) {
                let content = if block.content().is_empty() {
                    "<empty>".to_string()
                } else {
                    block.content().replace('\n', "\\n")
                };

                if block.is_deleted {
                    parts.push(format!("[DEL:{content}]"));
                } else {
                    parts.push(content);
                }

                curr = block.right();
            } else {
                parts.push("<broken-link>".to_string());
                break;
            }
        }

        if parts.is_empty() {
            "<empty>".to_string()
        } else {
            parts.join(" --- ")
        }
    }

    fn find_insert_position(&self, block: &Block) -> Result<Option<BlockId>, DocumentError> {
        use std::collections::HashSet;

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

        let mut seen: HashSet<BlockId> = HashSet::new();

        while let Some(curr_id) = scanning_id {
            if Some(curr_id) == right {
                break;
            }

            let curr_block = self
                .store
                .get(&curr_id)
                .ok_or(DocumentError::BlockNotFound(curr_id))?;

            let o_l = curr_block.origin_left;

            seen.insert(curr_id);

            let ol_is_left_of_ours = match (o_l, block.origin_left) {
                (None, Some(_)) => true,
                (Some(x), _) if Some(x) != block.origin_left && !seen.contains(&x) => true,
                _ => false,
            };

            if ol_is_left_of_ours {
                break;
            }

            if o_l == block.origin_left
                && block.id.client.value < curr_block.id.client.value
            {
                break;
            }

            left = Some(curr_id);
            scanning_id = curr_block.right();
        }

        Ok(left)
    }

    fn link_block(
        &mut self,
        block_id: BlockId,
        final_left: Option<BlockId>,
        final_right: Option<BlockId>,
    ) -> Result<(), DocumentError> {
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

        Ok(())
    }

    fn integrate(&mut self, block: Block) -> Result<BlockId, DocumentError> {
        let block_id = block.id;

        let final_left = self.find_insert_position(&block)?;
        let final_right = if let Some(l_id) = final_left {
            self.store
                .get(&l_id)
                .ok_or(DocumentError::BlockNotFound(l_id))?
                .right()
        } else {
            self.head
        };

        self.store.insert(block);
        self.link_block(block_id, final_left, final_right)?;

        Ok(block_id)
    }

    fn resolve_origins(
        &mut self,
        position: u64,
    ) -> Result<(Option<BlockId>, Option<BlockId>), DocumentError> {
        let (block, offset, tail_id) = self.get_block_and_offset_by_position(position);

        if let Some(block_id) = block {
            if offset == 0 {
                let block_ref = self
                    .store
                    .get(&block_id)
                    .ok_or(DocumentError::BlockNotFound(block_id))?;
                Ok((block_ref.left(), Some(block_id)))
            } else {
                self.split_block(block_id, offset)?;
                let left_ref = self
                    .store
                    .get(&block_id)
                    .ok_or(DocumentError::BlockNotFound(block_id))?;
                let origin_left_id =
                    BlockId::new(block_id.client, block_id.clock.advance(offset - 1));
                Ok((Some(origin_left_id), left_ref.right()))
            }
        } else if offset > 0 {
            Err(DocumentError::OutOfBounds(position))
        } else {
            Ok((tail_id, None))
        }
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

    pub fn remote_insert(&mut self, block: Block) -> Result<(), DocumentError> {
        match self.classify_block(&block) {
            Readiness::Duplicate => return Ok(()),
            Readiness::Pending => {
                self.pending_blocks.push(block);
                return Ok(());
            }
            Readiness::Ready => {}
        }

        let client = block.id.client;
        let end_clock = block.id.clock.value + block.len;

        self.pre_split_for_block(&block)?;
        self.integrate(block)?;
        self.state_vector.update(client, end_clock);

        self.drain_pending()?;
        Ok(())
    }

    fn classify_block(&self, block: &Block) -> Readiness {
        let seen = self.state_vector.get(&block.id.client);
        let clock = block.id.clock.value;

        if seen > clock {
            return Readiness::Duplicate;
        }
        if seen < clock {
            return Readiness::Pending;
        }
        if let Some(ol) = block.origin_left
            && !self.store.contains_key(&ol)
        {
            return Readiness::Pending;
        }
        if let Some(or) = block.origin_right
            && !self.store.contains_key(&or)
        {
            return Readiness::Pending;
        }
        Readiness::Ready
    }

    fn ensure_block_split_at(&mut self, id: BlockId) -> Result<(), DocumentError> {
        let block = match self.store.get(&id) {
            Some(b) => b,
            None => return Ok(()),
        };
        let block_start = block.id.clock.value;
        if block_start == id.clock.value {
            return Ok(());
        }
        let offset = id.clock.value - block_start;
        let block_id = block.id;
        self.split_block(block_id, offset)?;
        Ok(())
    }

    fn pre_split_for_block(&mut self, block: &Block) -> Result<(), DocumentError> {
        if let Some(ol) = block.origin_left {
            let split_point = BlockId::new(ol.client, ol.clock.advance(1));
            self.ensure_block_split_at(split_point)?;
        }
        if let Some(or_id) = block.origin_right {
            self.ensure_block_split_at(or_id)?;
        }
        Ok(())
    }

    fn drain_pending(&mut self) -> Result<(), DocumentError> {
        loop {
            let mut progress = false;

            let candidates: Vec<Block> = self.pending_blocks.drain(..).collect();
            let mut still_pending_blocks: Vec<Block> = Vec::new();
            for block in candidates {
                match self.classify_block(&block) {
                    Readiness::Ready => {
                        let client = block.id.client;
                        let end_clock = block.id.clock.value + block.len;
                        self.pre_split_for_block(&block)?;
                        self.integrate(block)?;
                        self.state_vector.update(client, end_clock);
                        progress = true;
                    }
                    Readiness::Duplicate => {
                        progress = true;
                    }
                    Readiness::Pending => {
                        still_pending_blocks.push(block);
                    }
                }
            }
            self.pending_blocks = still_pending_blocks;

            let candidate_ds: Vec<DeleteSet> = self.pending_delete_sets.drain(..).collect();
            let mut still_pending_ds: Vec<DeleteSet> = Vec::new();
            for ds in candidate_ds {
                let unapplied = self.try_apply_delete_set(&ds)?;
                if unapplied.is_empty() {
                    progress = true;
                } else {
                    still_pending_ds.push(unapplied);
                }
            }
            self.pending_delete_sets = still_pending_ds;

            if !progress {
                break;
            }
        }
        Ok(())
    }

    fn try_apply_delete_set(&mut self, remote: &DeleteSet) -> Result<DeleteSet, DocumentError> {
        let mut unapplied = DeleteSet::new();

        for (client, range) in remote.iter() {
            let mut current_clock = range.start;
            let end_clock = range.end();

            while current_clock < end_clock {
                let id = BlockId::new(*client, Clock::new(current_clock));

                let (block_start, block_len, block_id) = match self.store.get(&id) {
                    Some(b) => (b.id.clock.value, b.len, b.id),
                    None => {
                        unapplied.add(
                            BlockId::new(*client, Clock::new(current_clock)),
                            end_clock - current_clock,
                        );
                        break;
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

        Ok(unapplied)
    }

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

    pub fn apply_delete_set(&mut self, remote: &DeleteSet) -> Result<(), DocumentError> {
        let unapplied = self.try_apply_delete_set(remote)?;
        if !unapplied.is_empty() {
            self.pending_delete_sets.push(unapplied);
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

                let Some(block) = self.store.get(&id) else {
                    break;
                };
                let next_clock = block.id.clock.value + block.len;

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
            let old_right_block_id = block.right();
            let mut new_block: Block = Block::new(
                new_block_id,
                Some(block.id),
                block.origin_right,
                new_block_content,
            );

            new_block.is_deleted = block.is_deleted;
            new_block.set_right(old_right_block_id);

            block.set_right(Some(new_block_id));

            (old_right_block_id, new_block)
        };

        let new_block_id = new_block.id;
        self.store.insert(new_block);

        if let Some(right_id) = right_block_id {
            self.store
                .get_mut(&right_id)
                .ok_or(DocumentError::BlockNotFound(right_id))?
                .set_left(Some(new_block_id));
        }

        Ok(Some(new_block_id))
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
                return (Some(block.id), position, None);
            }
            position -= content_len;
            current_block = block.right().and_then(|id| self.store.get(&id));
        }

        (None, position, tail_id)
    }
}

#[cfg(test)]
mod tests;
