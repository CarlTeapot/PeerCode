use super::Document;
use crate::types::BlockId;

impl Document {
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

    /// Locate the block and intra-block offset that corresponds to a visible
    /// character position. Returns `(block_id, offset_within_block, tail_id)`.
    pub(super) fn get_block_and_offset_by_position(
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
