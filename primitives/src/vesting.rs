//! # Vesting primitives Module
//!
//! This module declares primitives for Vesting logic.
//! Constants declared: total amount of tokens for each bucket, vesting duration for each bucket,
//! the beginning of the vesting for each bucket.

use crate::currency::{GetDecimals, MNT};
use crate::Balance;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Zero, RuntimeDebug};

/// Vesting bucket type. Each bucket has its own rules for vesting.
/// Each type of bucket differs from each other in the total number of tokens, the duration of
/// the vesting, the beginning of the vesting.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum VestingBucket {
	Community,
	PrivateSale,
	PublicSale,
	MarketMaking,
	StrategicPartners,
	Marketing,
	Ecosystem,
	Team,
}

impl VestingBucket {
	/// Returns the beginning of the vesting in days.
	pub fn unlock_begins_in_days(&self) -> u8 {
		match self {
			VestingBucket::Team => 182,
			_ => u8::zero(),
		}
	}

	/// Returns the total number of tokens for each vesting bucket.
	pub fn total_amount(&self) -> Balance {
		match self {
			VestingBucket::Community => 50_032_400_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::PrivateSale => 10_001_000_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::PublicSale => 2_500_250_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::MarketMaking => 3_000_000_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::StrategicPartners => 1_949_100_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::Marketing => 4_000_400_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::Ecosystem => 4_499_880_u128 * 10_u128.saturating_pow(MNT.decimals()),
			VestingBucket::Team => 24_017_000_u128 * 10_u128.saturating_pow(MNT.decimals()),
		}
	}

	/// Returns the duration of the vesting in years for each bucket.
	pub fn vesting_duration(&self) -> u8 {
		match self {
			VestingBucket::Community => 5,
			VestingBucket::PrivateSale => 1,
			VestingBucket::PublicSale => 1,
			VestingBucket::MarketMaking => 0,
			VestingBucket::StrategicPartners => 2,
			VestingBucket::Marketing => 1,
			VestingBucket::Ecosystem => 4,
			VestingBucket::Team => 5,
		}
	}
}

/// The vesting schedule. Used to parse json file when creating a Genesis Block
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct VestingScheduleJson<AccountId, Balance> {
	/// The public key of the account that is included in the vesting
	pub account: AccountId,
	/// Vesting amount of tokens
	pub amount: Balance,
}

#[cfg(test)]
mod tests {
	use crate::VestingBucket;
	use sp_runtime::traits::Zero;

	#[test]
	fn check_vesting_buckets_begins() {
		assert_eq!(VestingBucket::Community.vesting_duration(), 5_u8);
		assert_eq!(VestingBucket::PrivateSale.vesting_duration(), 1_u8);
		assert_eq!(VestingBucket::PublicSale.vesting_duration(), 1_u8);
		assert_eq!(VestingBucket::MarketMaking.vesting_duration(), 0_u8);
		assert_eq!(VestingBucket::StrategicPartners.vesting_duration(), 2_u8);
		assert_eq!(VestingBucket::Marketing.vesting_duration(), 1_u8);
		assert_eq!(VestingBucket::Ecosystem.vesting_duration(), 4_u8);
		assert_eq!(VestingBucket::Team.vesting_duration(), 5_u8)
	}

	#[test]
	fn check_vesting_buckets_durations() {
		assert_eq!(VestingBucket::Team.unlock_begins_in_days(), 182_u8);
		assert_eq!(VestingBucket::Community.unlock_begins_in_days(), u8::zero());
		assert_eq!(VestingBucket::PrivateSale.unlock_begins_in_days(), u8::zero());
		assert_eq!(VestingBucket::PublicSale.unlock_begins_in_days(), u8::zero());
		assert_eq!(VestingBucket::MarketMaking.unlock_begins_in_days(), u8::zero());
		assert_eq!(VestingBucket::StrategicPartners.unlock_begins_in_days(), u8::zero());
		assert_eq!(VestingBucket::Marketing.unlock_begins_in_days(), u8::zero());
		assert_eq!(VestingBucket::Ecosystem.unlock_begins_in_days(), u8::zero())
	}

	#[test]
	fn check_vesting_buckets_total_amounts() {
		assert_eq!(
			VestingBucket::Community.total_amount(),
			50_032_400_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::PrivateSale.total_amount(),
			10_001_000_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::PublicSale.total_amount(),
			2_500_250_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::MarketMaking.total_amount(),
			3_000_000_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::StrategicPartners.total_amount(),
			1_949_100_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::Marketing.total_amount(),
			4_000_400_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::Ecosystem.total_amount(),
			4_499_880_000_000_000_000_000_000_u128
		);
		assert_eq!(
			VestingBucket::Team.total_amount(),
			24_017_000_000_000_000_000_000_000_u128
		);
		// The sum of total_amount all buckets must be equal to the const TOTAL_ALLOCATION,
		// declared in the file in runtime/constants.rs
		assert_eq!(
			VestingBucket::Community.total_amount()
				+ VestingBucket::PrivateSale.total_amount()
				+ VestingBucket::PublicSale.total_amount()
				+ VestingBucket::MarketMaking.total_amount()
				+ VestingBucket::StrategicPartners.total_amount()
				+ VestingBucket::Marketing.total_amount()
				+ VestingBucket::Ecosystem.total_amount()
				+ VestingBucket::Team.total_amount(),
			100_000_030_000_000_000_000_000_000
		);
	}
}
