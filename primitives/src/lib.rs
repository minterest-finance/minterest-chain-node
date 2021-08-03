#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
pub use currency::CurrencyId;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	FixedI128, FixedU128, MultiSignature, RuntimeDebug,
};
pub use vesting::{VestingBucket, VestingScheduleJson};

pub mod arithmetic;
pub mod constants;
pub mod currency;
pub mod vesting;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Signed version of Balance
pub type Amount = i128;

/// Exchange Rate
pub type Rate = FixedU128;

/// Token Price
pub type Price = FixedU128;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Opaque, encoded, unchecked extrinsic.
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

/// An instant or duration in time.
pub type Moment = u64;

/// Decimal representation of interest. Signed.
pub type Interest = FixedI128;

/// Chainlink Feed Id
pub type ChainlinkFeedId = u32;

/// Chainlink value to represent oracle price in USD.
/// Expect all prices will be provided with 18 decimals.
pub type ChainlinkPriceValue = u128;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Operation {
	Deposit,
	Redeem,
	Borrow,
	Repay,
	Transfer,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum DataProviderId {
	Aggregated = 0,
	Minterest = 1,
}

/// Error which may occur while executing the off-chain code.
#[derive(PartialEq, Eq)]
pub enum OffchainErr {
	OffchainLock,
	NotValidator,
	GetUsersWithInsolventLoanFailed,
	BuildUserLoanStateFailed,
	NotAllLoansLiquidated,
	LiquidateTransactionFailed,
	PoolsBalancingError,
	PoolsBalancingIsOff,
	FailReceivingOraclePrice,
	ChainlinkFeedNotExists,
	NumOverflow,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::OffchainLock => write!(fmt, "Failed to get or extend lock"),
			OffchainErr::NotValidator => write!(fmt, "Not validator"),
			OffchainErr::GetUsersWithInsolventLoanFailed => write!(fmt, "Failed to get all users with insolvent loan"),
			OffchainErr::BuildUserLoanStateFailed => {
				write!(fmt, "Failed to calculate and build the user's loan state.")
			}
			OffchainErr::NotAllLoansLiquidated => write!(fmt, "Not all insolvent loans have been liquidated"),
			OffchainErr::LiquidateTransactionFailed => write!(fmt, "Error executing liquidation extrinsic"),
			OffchainErr::PoolsBalancingError => write!(fmt, "Pools balancing error"),
			OffchainErr::PoolsBalancingIsOff => write!(fmt, "Pools balancing switched off"),
			OffchainErr::FailReceivingOraclePrice => write!(fmt, "Receiving oracle price is failed"),
			OffchainErr::ChainlinkFeedNotExists => write!(fmt, "Can't retrieve feed for enabled currency"),
			OffchainErr::NumOverflow => write!(fmt, "Number overflow"),
		}
	}
}
