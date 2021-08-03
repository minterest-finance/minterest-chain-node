//! Runtime API definition for prices pallet.
//! Here we declare the runtime API. It is implemented in the `impl` block in
//! runtime amalgamator file (the `runtime/src/lib.rs`)
//!
//! Corresponding RPC declaration: `pallets/prices/rpc/src/lib.rs`

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use minterest_primitives::{OriginalAsset, Price};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait PricesRuntimeApi
	{
		fn  get_current_price(currency_id: OriginalAsset) -> Option<Price>;
		fn  get_all_locked_prices() -> Vec<(OriginalAsset, Option<Price>)>;
		fn  get_all_freshest_prices() -> Vec<(OriginalAsset, Option<Price>)>;
	}
}
