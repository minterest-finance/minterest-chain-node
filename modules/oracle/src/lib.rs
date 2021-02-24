#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::pallet_prelude::*;
use minterest_primitives::{CurrencyId, Price};
use sp_runtime::DispatchError;
use sp_std::result;

pub use module::*;

type PriceResult = result::Result<Price, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}
impl<T: Config> Pallet<T> {
	pub fn get_underlying_price(_underlying_asset_id: CurrencyId) -> PriceResult {
		let price_two_dollars = 2_00u128 * 10_000_000_000_000_000;
		Ok(Price::from_inner(price_two_dollars)) // Price = 2.00 USD
	}
}