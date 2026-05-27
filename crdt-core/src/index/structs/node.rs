use crate::index::constants::NODE_CHILDREN;
use crate::index::structs::handles::NodeIdx;

#[derive(Debug, Clone, Copy)]
pub(in crate::index) struct ChildSlot {
    pub idx: u32,
    pub visible_len: u64,
}

#[derive(Debug)]
pub(in crate::index) struct Node {
    pub child_slots: [Option<ChildSlot>; NODE_CHILDREN],
    pub num_children: u8,
    pub parent: Option<NodeIdx>,
    pub is_leaf_parent: bool,
}

impl Node {
    pub fn new(is_leaf_parent: bool) -> Self {
        Node {
            child_slots: [None; NODE_CHILDREN],
            num_children: 0,
            parent: None,
            is_leaf_parent,
        }
    }

    pub fn visible_len(&self) -> u64 {
        self.child_slots
            .iter()
            .filter_map(|s| s.map(|s| s.visible_len))
            .sum()
    }
}
