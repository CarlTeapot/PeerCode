#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bitcode::Encode, bitcode::Decode,
)]
pub struct ClientId {
    pub value: u64,
}

impl ClientId {
    pub fn new(value: u64) -> Self {
        ClientId { value }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bitcode::Encode, bitcode::Decode,
)]
pub struct Clock {
    pub value: u64,
}

impl Clock {
    pub const ZERO: Clock = Clock { value: 0 };

    pub fn new(value: u64) -> Self {
        Clock { value }
    }

    pub fn advance(&self, by: u64) -> Clock {
        Clock {
            value: self.value + by,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bitcode::Encode, bitcode::Decode)]
pub struct BlockId {
    pub client: ClientId,
    pub clock: Clock,
}

impl BlockId {
    pub fn new(client: ClientId, clock: Clock) -> Self {
        BlockId { client, clock }
    }

    pub fn at_offset(&self, offset: u64) -> BlockId {
        BlockId {
            client: self.client,
            clock: Clock {
                value: self.clock.value + offset,
            },
        }
    }
}

impl PartialOrd for BlockId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BlockId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.clock
            .cmp(&other.clock)
            .reverse()
            .then(self.client.cmp(&other.client))
    }
}
