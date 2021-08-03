//! Autogenerated weights for whitelist_module
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-07-03, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/minterest
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=whitelist_module
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --output=./runtime/standalone/src/weights/whitelist.rs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for whitelist_module.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> whitelist_module::WeightInfo for WeightInfo<T> {
	fn add_member(m: u32) -> Weight {
		(27_276_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((67_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn remove_member(m: u32) -> Weight {
		(27_629_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((77_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn switch_whitelist_mode() -> Weight {
		(19_344_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}