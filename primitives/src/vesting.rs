use crate::currency::GetDecimals;
use crate::currency::MNT;
use crate::Balance;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::Zero;
use sp_runtime::RuntimeDebug;

/// Vesting bucket type. Each bucket has its own rules for vesting.
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
