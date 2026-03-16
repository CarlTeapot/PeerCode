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

    content: String,

    is_deleted: bool,

    len: u64,
}

impl Block {
    pub fn new(
        id: BlockId,
        origin_left: Option<BlockId>,
        origin_right: Option<BlockId>,
        content: String,
    ) -> Self {
        let len = content.chars().count() as u64;

        Block {
            id,
            origin_left,
            origin_right,
            left: origin_left,
            right: origin_right,
            content,
            is_deleted: false,
            len,
        }
    }

    pub fn left(&self) -> Option<BlockId> {
        self.left
    }

    pub fn right(&self) -> Option<BlockId> {
        self.right
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn len(&self) -> u64 {
        self.len
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
