//! A set of constant values used in runtime.

use minterest_primitives::Rate;

/// Money matters.
pub mod currency {
	use minterest_primitives::Balance;

	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const CENTS: Balance = DOLLARS / 100;
	pub const MILLICENTS: Balance = CENTS / 1000;
}

/// Time.
pub mod time {
	use minterest_primitives::BlockNumber;
	pub const MILLISECS_PER_BLOCK: u64 = 6000;

	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const BLOCKS_PER_YEAR: u128 = 365 * DAYS as u128;
	// BLOCKS_PER_YEAR has to be 5256000
}

/// A maximum number of admins. When membership reaches this number, no new members may join.
pub const MAX_MEMBERS: u8 = 16;

pub const MAX_BORROW_CAP: minterest_primitives::Balance = 1_000_000_000_000_000_000_000_000;

/// Initial exchange rate: 100%
pub const INITIAL_EXCHANGE_RATE: Rate = Rate::from_inner(1_000_000_000_000_000_000);
