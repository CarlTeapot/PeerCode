use crate::types::BlockId;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,

    // Original neighbors.
    pub origin_left: Option<BlockId>,
    pub origin_right: Option<BlockId>,

    // Current neighbors.
    left: Option<BlockId>,
    right: Option<BlockId>,

    pub content: String,

    is_deleted: bool,
}

impl Block {
    pub fn new(
        id: BlockId,
        origin_left: Option<BlockId>,
        origin_right: Option<BlockId>,
        content: String,
    ) -> Self {
        Block {
            id,
            origin_left,
            origin_right,
            left: origin_left,
            right: origin_right,
            content,
            is_deleted: false,
        }
    }

    pub fn left(&self) -> Option<BlockId> {
        self.left
    }

    pub fn right(&self) -> Option<BlockId> {
        self.right
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn len(&self) -> u64 {
        self.content.chars().count() as u64
    }

    pub fn is_deleted(&self) -> bool {
        self.is_deleted
    }

    pub fn delete(&mut self) {
        self.is_deleted = true;
    }

    // empty for now , will implement later when we have the basic structure in place
    //pub fn split(self, offset: u64) -> (Block, Block) {}

    //pub fn squash(self, other: Block) -> Result<Block, (Block, Block)> {}
}
