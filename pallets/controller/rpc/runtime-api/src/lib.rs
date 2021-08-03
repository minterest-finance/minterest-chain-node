//! Runtime API definition for controller pallet.
//! Here we declare the runtime API. It is implemented in the `impl` block in
//! runtime amalgamator file (the `runtime/src/lib.rs`)
//!
//! Corresponding RPC declaration: `pallets/controller/rpc/src/lib.rs`

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use codec::{Codec, Decode, Encode};
use minterest_primitives::{Amount, Balance, OriginalAsset, Interest, Rate};
use sp_core::RuntimeDebug;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct PoolState {
	pub exchange_rate: Rate,
	pub borrow_rate: Rate,
	pub supply_rate: Rate,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct UserPoolBalanceData {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_supply_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_borrowed_in_usd: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct HypotheticalLiquidityData {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub liquidity_in_usd: Amount,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct BalanceInfo {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub amount: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct ProtocolTotalValue {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub pool_total_supply_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub pool_total_borrow_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub tvl_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub pool_total_protocol_interest_in_usd: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct UserData {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_collateral_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_supply_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_borrow_in_usd: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_supply_apy: Rate,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub total_borrow_apy: Rate,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub net_apy: Rate,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(t: &T, serializer: S) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(deserializer: D) -> Result<T, D::Error> {
	let s = String::deserialize(deserializer)?;
	s.parse::<T>()
		.map_err(|_| serde::de::Error::custom("Parse from string failed"))
}

sp_api::decl_runtime_apis! {
	pub trait ControllerRuntimeApi<AccountId>
	where
		AccountId: Codec,
	{
		fn get_user_data(account_id: AccountId) -> Option<UserData>;

		fn get_protocol_total_values() -> Option<ProtocolTotalValue>;

		fn liquidity_pool_state(pool_id: OriginalAsset) -> Option<PoolState>;

		fn get_pool_utilization_rate(pool_id: OriginalAsset) -> Option<Rate>;

		fn get_user_total_supply_and_borrow_balance_in_usd(account_id: AccountId) -> Option<UserPoolBalanceData>;

		fn get_hypothetical_account_liquidity(account_id: AccountId) -> Option<HypotheticalLiquidityData>;

		fn is_admin(caller: AccountId) -> Option<bool>;

		fn get_user_total_collateral(account_id: AccountId) -> Option<BalanceInfo>;

		fn get_user_borrow_per_asset(
			account_id: AccountId,
			pool_id: OriginalAsset,
		) -> Option<BalanceInfo>;

		fn get_user_underlying_balance_per_asset(
			account_id: AccountId,
			pool_id: OriginalAsset,
		) -> Option<BalanceInfo>;

		fn pool_exists(pool_id: OriginalAsset) -> bool;

		fn get_user_total_supply_borrow_and_net_apy(account_id: AccountId) -> Option<(Interest, Interest, Interest)>;

		fn get_user_total_borrow_usd(
			account_id: AccountId,
		) -> Option<BalanceInfo>;
	}
}
