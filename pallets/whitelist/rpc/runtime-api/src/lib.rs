//! Runtime API definition for whitelist module.
//! Here we declare the runtime API. It is implemented in the `impl` block in
//! runtime amalgamator file (the `runtime/src/lib.rs`)
//!
//! Corresponding RPC declaration: `pallets/whitelist/rpc/src/lib.rs`

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use codec::Codec;
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	pub trait WhitelistRuntimeApi<AccountId>
	where
		AccountId: Codec,
	{
		fn is_whitelist_member(account_id: AccountId) -> Option<bool>;
	}
}
