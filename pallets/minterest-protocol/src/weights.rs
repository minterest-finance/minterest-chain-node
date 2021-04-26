// This file is part of Minterest.

// Copyright (C) 2021 Minterest finance.

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Autogenerated weights for minterest_protocol
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-04-26, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/minterest
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=minterest_protocol
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --output=./pallets/minterest-protocol/src/weights.rs
// --template=./templates/weight-template-for-pallet.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for minterest_protocol.
pub trait WeightInfo {
	fn deposit_underlying() -> Weight;
	fn redeem() -> Weight;
	fn redeem_underlying() -> Weight;
	fn redeem_wrapped() -> Weight;
	fn borrow() -> Weight;
	fn repay() -> Weight;
	fn repay_all() -> Weight;
	fn repay_on_behalf() -> Weight;
	fn transfer_wrapped() -> Weight;
	fn enable_is_collateral() -> Weight;
	fn disable_is_collateral() -> Weight;
	fn claim_mnt() -> Weight;
}

/// Weights for minterest_protocol using the Minterest node and recommended hardware.
pub struct MinterestWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for MinterestWeight<T> {
	fn deposit_underlying() -> Weight {
		(513_126_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(17 as Weight))
			.saturating_add(T::DbWeight::get().writes(11 as Weight))
	}
	fn redeem() -> Weight {
		(1_087_532_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(39 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}
	fn redeem_underlying() -> Weight {
		(800_391_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(39 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}
	fn redeem_wrapped() -> Weight {
		(1_044_682_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(39 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}
	fn borrow() -> Weight {
		(968_715_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(37 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	fn repay() -> Weight {
		(438_995_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	fn repay_all() -> Weight {
		(428_870_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	fn repay_on_behalf() -> Weight {
		(423_295_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	fn transfer_wrapped() -> Weight {
		(880_860_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(40 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	fn enable_is_collateral() -> Weight {
		(110_050_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn disable_is_collateral() -> Weight {
		(436_548_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(30 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn claim_mnt() -> Weight {
		(2_002_677_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(35 as Weight))
			.saturating_add(T::DbWeight::get().writes(15 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn deposit_underlying() -> Weight {
		(513_126_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(17 as Weight))
			.saturating_add(RocksDbWeight::get().writes(11 as Weight))
	}
	fn redeem() -> Weight {
		(1_087_532_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(39 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}
	fn redeem_underlying() -> Weight {
		(800_391_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(39 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}
	fn redeem_wrapped() -> Weight {
		(1_044_682_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(39 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}
	fn borrow() -> Weight {
		(968_715_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(37 as Weight))
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
	}
	fn repay() -> Weight {
		(438_995_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(13 as Weight))
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
	}
	fn repay_all() -> Weight {
		(428_870_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(13 as Weight))
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
	}
	fn repay_on_behalf() -> Weight {
		(423_295_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(13 as Weight))
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
	}
	fn transfer_wrapped() -> Weight {
		(880_860_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(40 as Weight))
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
	}
	fn enable_is_collateral() -> Weight {
		(110_050_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn disable_is_collateral() -> Weight {
		(436_548_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(30 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn claim_mnt() -> Weight {
		(2_002_677_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(35 as Weight))
			.saturating_add(RocksDbWeight::get().writes(15 as Weight))
	}
}
