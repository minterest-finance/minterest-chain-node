//! A set of constant values used in runtime.

use crate::constants::currency::DOLLARS;
use crate::{Balance, Rate};

pub mod time {
	use crate::BlockNumber;

	pub const MILLISECS_PER_BLOCK: u64 = 6000;

	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const BLOCKS_PER_YEAR: u128 = 365 * DAYS as u128;
	// BLOCKS_PER_YEAR has to be 5256000
}

pub mod currency {
	use crate::Balance;

	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const CENTS: Balance = DOLLARS / 100;
	pub const MILLICENTS: Balance = CENTS / 1000;
}

pub mod liquidation {
	use crate::constants::currency::DOLLARS;
	use crate::Balance;

	/// Minimal sum for partial liquidation.
	/// Loans with amount below this parameter will be liquidate in full.
	pub const PARTIAL_LIQUIDATION_MIN_SUM: Balance = 100_000 * DOLLARS;

	/// The maximum number of partial liquidations a user has. After reaching this parameter,
	/// a complete liquidation occurs.
	pub const PARTIAL_LIQUIDATION_MAX_ATTEMPTS: u8 = 3_u8;
}

pub mod fee {
	use crate::Balance;
	use frame_support::weights::{
		constants::ExtrinsicBaseWeight, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	};
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

pub const MAX_BORROW_CAP: Balance = 1_000_000_000_000_000_000_000_000;
pub const PROTOCOL_INTEREST_TRANSFER_THRESHOLD: Balance = 1_000_000_000_000_000_000_000;

/// Initial exchange rate: 100%
pub const INITIAL_EXCHANGE_RATE: Rate = Rate::from_inner(1_000_000_000_000_000_000);

/// Total allocation of MNT tokens
pub const TOTAL_ALLOCATION: Balance = 100_000_030 * DOLLARS;
