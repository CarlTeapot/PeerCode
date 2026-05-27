use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::node::{ChildSlot, Node};
use crate::index::structs::storage::Storage;

impl Storage {
    pub fn make_root_node_for_two_leaves(
        &mut self,
        left: LeafIdx,
        left_visible: u64,
        right: LeafIdx,
        right_visible: u64,
    ) -> NodeIdx {
        let mut node = Node::new(true);
        node.child_slots[0] = Some(ChildSlot {
            idx: left.0,
            visible_len: left_visible,
        });
        node.child_slots[1] = Some(ChildSlot {
            idx: right.0,
            visible_len: right_visible,
        });
        node.num_children = 2;
        let node_idx = NodeIdx(self.nodes.len() as u32);
        self.nodes.push(node);
        self.leaves[left.0 as usize].parent = Some(node_idx);
        self.leaves[right.0 as usize].parent = Some(node_idx);
        node_idx
    }

    pub fn make_root_node_for_two_nodes(
        &mut self,
        left: NodeIdx,
        left_visible: u64,
        right: NodeIdx,
        right_visible: u64,
    ) -> NodeIdx {
        let mut node = Node::new(false);
        node.child_slots[0] = Some(ChildSlot {
            idx: left.0,
            visible_len: left_visible,
        });
        node.child_slots[1] = Some(ChildSlot {
            idx: right.0,
            visible_len: right_visible,
        });
        node.num_children = 2;
        let node_idx = NodeIdx(self.nodes.len() as u32);
        self.nodes.push(node);
        self.nodes[left.0 as usize].parent = Some(node_idx);
        self.nodes[right.0 as usize].parent = Some(node_idx);
        node_idx
    }
}
