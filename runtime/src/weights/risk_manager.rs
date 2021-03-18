//! Autogenerated weights for risk_manager
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-03-18, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 128

// Executed Command:
// ./target/release/minterest
// benchmark
// --dev
// --steps=50
// --repeat=20
// --pallet=risk_manager
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --output=./runtime/src/weights/risk_manager.rs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for risk_manager.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> risk_manager::WeightInfo for WeightInfo<T> {
	fn set_max_attempts() -> Weight {
		(32_299_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_min_sum() -> Weight {
		(32_153_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_threshold() -> Weight {
		(31_824_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_liquidation_incentive() -> Weight {
		(31_528_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn liquidate() -> Weight {
		(1_289_303_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(78 as Weight))
			.saturating_add(T::DbWeight::get().writes(35 as Weight))
	}
}
