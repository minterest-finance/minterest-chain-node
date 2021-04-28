//! Runtime API definition for prices pallet.
//! Here we declare the runtime API. It is implemented it the `impl` block in
//! runtime amalgamator file (the `runtime/src/lib.rs`)
//!
//! Corresponding RPC declaration: `pallets/prices/rpc/src/lib.rs`

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use minterest_primitives::{CurrencyId, Price};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait PricesApi
	{
		fn  get_current_price(currency_id: CurrencyId) -> Option<Price>;
		fn  get_all_locked_prices() -> Vec<(CurrencyId, Option<Price>)>;
	}
}
