#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(pub u64);

impl ClientId {
    pub const HOST: ClientId = ClientId(0);

    pub fn is_host(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Clock(pub u64);

impl Clock {
    pub const ZERO: Clock = Clock(0);

    pub fn advance(&self, by: u64) -> Clock {
        Clock(self.0 + by)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            clock: Clock(self.clock.0 + offset),
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
