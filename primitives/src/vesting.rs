//! # Vesting primitives Module
//!
//! This module declares primitives for Vesting logic.
//! Constants declared: total amount of tokens for each bucket, vesting duration for each bucket,
//! the beginning of the vesting for each bucket.

#![allow(clippy::vec_init_then_push)]

use crate::{AccountId, Balance, constants::currency::DOLLARS};
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::{prelude::Vec, vec};

macro_rules! create_vesting_bucket_info {
	($(#[$meta:meta])*
	$vis:vis enum VestingBucket {
		$($bucket_type:ident,)*
	}) => {
		$(#[$meta])*
        $vis enum VestingBucket {
            $($bucket_type,)*
        }

		impl VestingBucket {
			/// This associated function is implemented for the frontend part of the protocol.
			/// Returns information for each vesting bucket:
			/// (vesting bucket type, vesting_duration, unlock_begins_in_days, total_amount)
			pub fn get_vesting_buckets_info() -> Vec<(VestingBucket, u8, u8, Balance)> {
				let mut enabled_buckets: Vec<(VestingBucket, u8, u8, Balance)> = vec![];
				$(
					enabled_buckets.push((
						VestingBucket::$bucket_type,
						VestingBucket::$bucket_type.vesting_duration(),
						VestingBucket::$bucket_type.unlock_begins_in_days(),
						VestingBucket::$bucket_type.total_amount(),
					));
				)*
				enabled_buckets
			}
		}
	}
}

create_vesting_bucket_info! {
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
			VestingBucket::Community => 50_032_400_u128 * DOLLARS,
			VestingBucket::PrivateSale => 10_001_000_u128 * DOLLARS,
			VestingBucket::PublicSale => 2_500_250_u128 * DOLLARS,
			VestingBucket::MarketMaking => 3_000_000_u128 * DOLLARS,
			VestingBucket::StrategicPartners => 1_949_100_u128 * DOLLARS,
			VestingBucket::Marketing => 4_000_400_u128 * DOLLARS,
			VestingBucket::Ecosystem => 4_499_880_u128 * DOLLARS,
			VestingBucket::Team => 24_017_000_u128 * DOLLARS,
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

	/// Returns vesting bucket account ID.
	pub fn bucket_account_id(&self) -> Option<AccountId> {
		match self {
			VestingBucket::Marketing => {
				// 5DeU3wfJJqNsEmrhLy8Tbq3CaK3RfhEnUs3iXM5yJokG6iWT
				Some(hex_literal::hex!["45fc1a76497800f75b283f6df15933a51c8c16c050c5c6156a9f7003781e6a7b"].into())
			}
			VestingBucket::StrategicPartners => {
				// 5DJpUxkx2TDrS2igf3wejnXLJHUqKzo9m3x76VWvai6NR6zF
				Some(hex_literal::hex!["36fff92edbfe9a75ae88915e5c2e019ff65bedff0bf11cdb5921863283f8bdb1"].into())
			}
			VestingBucket::Team => {
				// 5GfxgwrBUmYMKu6AeBAEzpYwC5TxrxRLJdg9oh2HrVnXQRs6
				Some(hex_literal::hex!["cbd474041eb2dd3d3bc63d411bdf25bd3b2df3e2b7ab4774bcd1c4cf5ce685ef"].into())
			}
			_ => None,
		}
	}

	/// Returns a Boolean value indicating whether the schedule from this vesting bucket can be
	/// removed or added.
	pub fn is_manipulated_bucket(&self) -> bool {
		*self == VestingBucket::Team || *self == VestingBucket::Marketing || *self == VestingBucket::StrategicPartners
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
	use crate::constants::TOTAL_ALLOCATION;
	use crate::{AccountId, VestingBucket};
	use sp_runtime::traits::Zero;

	#[test]
	fn check_vesting_buckets_durations() {
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
	fn check_vesting_buckets_begins() {
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
			TOTAL_ALLOCATION
		);
	}

	#[test]
	fn check_vesting_buckets_accounts_should_work() {
		use sp_core::crypto::Ss58Codec;
		assert_eq!(
			VestingBucket::Marketing.bucket_account_id(),
			Some(AccountId::from_string("5DeU3wfJJqNsEmrhLy8Tbq3CaK3RfhEnUs3iXM5yJokG6iWT").unwrap())
		);
		assert_eq!(
			VestingBucket::StrategicPartners.bucket_account_id(),
			Some(AccountId::from_string("5DJpUxkx2TDrS2igf3wejnXLJHUqKzo9m3x76VWvai6NR6zF").unwrap())
		);
		assert_eq!(
			VestingBucket::Team.bucket_account_id(),
			Some(AccountId::from_string("5GfxgwrBUmYMKu6AeBAEzpYwC5TxrxRLJdg9oh2HrVnXQRs6").unwrap())
		);
		assert_eq!(VestingBucket::Ecosystem.bucket_account_id(), None);
	}

	#[test]
	fn check_vesting_buckets_info() {
		assert_eq!(
			VestingBucket::get_vesting_buckets_info(),
			vec![
				(
					VestingBucket::Community,
					5_u8,
					0_u8,
					50_032_400_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::PrivateSale,
					1_u8,
					0_u8,
					10_001_000_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::PublicSale,
					1_u8,
					0_u8,
					2_500_250_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::MarketMaking,
					0_u8,
					0_u8,
					3_000_000_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::StrategicPartners,
					2_u8,
					0_u8,
					1_949_100_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::Marketing,
					1_u8,
					0_u8,
					4_000_400_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::Ecosystem,
					4_u8,
					0_u8,
					4_499_880_000_000_000_000_000_000_u128
				),
				(
					VestingBucket::Team,
					5_u8,
					182_u8,
					24_017_000_000_000_000_000_000_000_u128
				)
			]
		);
	}
}
