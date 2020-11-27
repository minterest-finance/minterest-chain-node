//! A set of constant values used in runtime.

/// Time.
pub mod time {
    use minterest_primitives::{BlockNumber};
    pub const MILLISECS_PER_BLOCK: u64 = 6000;

    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}
