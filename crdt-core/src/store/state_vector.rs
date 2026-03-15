use crate::types::{BlockId, ClientId};
use std::collections::HashMap;

// StateVector struct to represent the state of seen blocks for each client.
// If sv[client] = N, it means we've seen all blocks from [0, N) clock values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StateVector(pub HashMap<ClientId, u64>);

impl StateVector {
    pub fn new() -> Self {
        StateVector(HashMap::new())
    }

    /// Get the next expected clock for a given client.
    pub fn get(&self, client: &ClientId) -> u64 {
        *self.0.get(client).unwrap_or(&0)
    }

    /// Update the state vector after integrating a block.
    pub fn update(&mut self, client: ClientId, end_clock: u64) {
        let entry = self.0.entry(client).or_insert(0);
        if end_clock > *entry {
            *entry = end_clock;
        }
    }

    /// Returns true if we have seen the given block.
    pub fn has_block(&self, id: &BlockId, len: u64) -> bool {
        self.get(&id.client) >= id.clock.0 + len
    }

    /// Returns true if we can integrate the given block
    pub fn can_integrate(&self, id: &BlockId) -> bool {
        let seen = self.get(&id.client);
        seen == id.clock.0
    }

    // empty for now , will implement later when we have the basic structure in place
    //pub fn diff_from(&self, remote: &StateVector) -> Vec<(ClientId, u64)> {}

    //pub fn merge(&mut self, other: &StateVector) {}

    //pub fn encode(&self) -> Vec<u8> {}

    //pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {}
}
