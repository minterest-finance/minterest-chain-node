//! A set of constant values used in runtime.

use crate::DOLLARS;
use minterest_primitives::{Balance, Rate};

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

	// The MntSpeed update period.
	pub const REFRESH_SPEED_PERIOD: BlockNumber = 5;
}

pub const MAX_BORROW_CAP: minterest_primitives::Balance = 1_000_000_000_000_000_000_000_000;
pub const PROTOCOL_INTEREST_TRANSFER_THRESHOLD: minterest_primitives::Balance = 1_000_000_000_000_000_000_000;

/// Initial exchange rate: 100%
pub const INITIAL_EXCHANGE_RATE: Rate = Rate::from_inner(1_000_000_000_000_000_000);

/// Total allocation of MNT tokens
pub const TOTAL_ALLOCATION: Balance = 100_000_000 * DOLLARS;

pub mod fee {
	use frame_support::weights::constants::ExtrinsicBaseWeight;
	use frame_support::weights::{WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial};
	use minterest_primitives::Balance;
	use smallvec::smallvec;
	use sp_runtime::Perbill;

	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// Extrinsic base weight is mapped to 0.43 MNT
			let p = 426_974_397_875_000_000;
			let q = Balance::from(ExtrinsicBaseWeight::get()); // 125_000_000
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::zero(), // zero
				coeff_integer: p / q,        // 3_415_795_183
			}]
		}
	}
}
