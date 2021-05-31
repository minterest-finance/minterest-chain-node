use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
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

/// The vesting schedule. Used to parse json file when creating a Genesis Block
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct VestingScheduleJson<AccountId, BlockNumber, Balance> {
	/// The public key of the account that is included in the vesting
	pub account: AccountId,
	/// Vesting starting block
	pub start: BlockNumber,
	/// Number of blocks between vest
	pub period: BlockNumber,
	/// Number of vest
	pub period_count: u32,
	/// Amount of tokens to release per vest
	pub per_period: Balance,
}

pub const COMMUNITY_YEARS_VESTING: u8 = 1;
pub const PRIVATE_SALE_YEARS_VESTING: u8 = 1;
pub const PUBLIC_SALE_YEARS_VESTING: u8 = 1;
pub const MARKET_MAKING_YEARS_VESTING: u8 = 0;
pub const STRATEGIC_PARTNERS_YEARS_VESTING: u8 = 0;
pub const MARKETING_YEARS_VESTING: u8 = 0;
pub const ECOSYSTEM_YEARS_VESTING: u8 = 0;
pub const TEAM_PARTNERS_YEARS_VESTING: u8 = 0;
