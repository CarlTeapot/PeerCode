use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::storage::Storage;

impl Storage {
    pub fn descend_leftmost_leaf(&self, start: NodeIdx) -> LeafIdx {
        let mut current = start;
        loop {
            let node = &self.nodes[current.0 as usize];
            debug_assert!(node.num_children > 0);
            let first = node.child_slots[0].expect("at least one child");
            if node.is_leaf_parent {
                return LeafIdx(first.idx);
            } else {
                current = NodeIdx(first.idx);
            }
        }
    }

    pub fn bubble_visible_len_delta(&mut self, leaf_idx: LeafIdx, delta: i64) {
        let mut parent_opt = self.leaves[leaf_idx.0 as usize].parent;
        let mut child_idx_u32 = leaf_idx.0;

        while let Some(parent) = parent_opt {
            let node = &mut self.nodes[parent.0 as usize];
            let mut found = false;
            for slot in node.child_slots.iter_mut() {
                if let Some(s) = slot.as_mut()
                    && s.idx == child_idx_u32
                {
                    s.visible_len = (s.visible_len as i64 + delta) as u64;
                    found = true;
                    break;
                }
            }
            debug_assert!(found, "parent slot pointing to child not found");
            parent_opt = node.parent;
            child_idx_u32 = parent.0;
        }
    }

    pub fn bubble_visible_len_delta_from_node(&mut self, node_idx: NodeIdx, delta: i64) {
        let mut parent_opt = self.nodes[node_idx.0 as usize].parent;
        let mut child_idx_u32 = node_idx.0;
        while let Some(parent) = parent_opt {
            let node = &mut self.nodes[parent.0 as usize];
            for slot in node.child_slots.iter_mut() {
                if let Some(s) = slot.as_mut()
                    && s.idx == child_idx_u32
                {
                    s.visible_len = (s.visible_len as i64 + delta) as u64;
                    break;
                }
            }
            parent_opt = node.parent;
            child_idx_u32 = parent.0;
        }
    }
}
