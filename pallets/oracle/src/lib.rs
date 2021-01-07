#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use minterest_primitives::{CurrencyId, Price};
use sp_runtime::DispatchError;
use sp_std::result;

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Exchange {
	}
}

decl_event!(
	pub enum Event {}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;
	}
}

type PriceResult = result::Result<Price, DispatchError>;

impl<T: Trait> Module<T> {
	pub fn get_underlying_price(_underlying_asset_id: CurrencyId) -> PriceResult {
		let price_nine_dollars = 2_00u128 * 10_000_000_000_000_000;
		Ok(Price::from_inner(price_nine_dollars)) // Price = 2.00 USD
	}
}
