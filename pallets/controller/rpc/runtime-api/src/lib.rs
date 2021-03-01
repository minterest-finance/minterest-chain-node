//! Runtime API definition for controller pallet.

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use codec::{Decode, Encode};
use minterest_primitives::{AccountId, Balance, CurrencyId, Rate};
use sp_core::RuntimeDebug;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct PoolState {
	pub exchange_rate: Rate,
	pub borrow_rate: Rate,
	pub supply_rate: Rate,
}

// Here we declare the runtime API. It is implemented it the `impl` block in
// runtime amalgamator file (the `runtime/src/lib.rs`)
sp_api::decl_runtime_apis! {
	pub trait ControllerApi {
		fn liquidity_pool_state(pool_id: CurrencyId) -> Option<PoolState>;

		fn get_total_supply_and_borrowed_usd_balance(account_id: AccountId) -> Option<(Balance, Balance)>;
	}
}
