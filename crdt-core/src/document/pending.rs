use super::Document;
use crate::error::DocumentError;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, Clock};

/// Classification of an incoming remote block relative to local state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readiness {
    /// Every prerequisite is present; the block can be integrated now.
    Ready,
    /// The block depends on something not yet seen
    Pending,
    /// We have already integrated this clock range.
    Duplicate,
}

impl Document {
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

    pub fn apply_delete_set(&mut self, remote: &DeleteSet) -> Result<(), DocumentError> {
        let unapplied = self.try_apply_delete_set(remote)?;
        if !unapplied.is_empty() {
            self.pending_delete_sets.push(unapplied);
        }
        self.seen_delete_set.merge(remote);
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

    /// Repeatedly drain pending blocks and pending delete sets until a pass
    /// makes no further progress.
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

    /// Attempt to apply every range in `remote`.
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
}
