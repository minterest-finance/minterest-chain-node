#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use minterest_primitives::{CurrencyId, Price};
use sp_runtime::DispatchError;
use sp_std::result;

pub trait Config: frame_system::Config {
	type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
	trait Store for Module<T: Config> as Exchange {
	}
}

decl_event!(
	pub enum Event {}
);

decl_error! {
	pub enum Error for Module<T: Config> {
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;
	}
}

type PriceResult = result::Result<Price, DispatchError>;

impl<T: Config> Module<T> {
	pub fn get_underlying_price(_underlying_asset_id: CurrencyId) -> PriceResult {
		let price_two_dollars = 2_00u128 * 10_000_000_000_000_000;
		Ok(Price::from_inner(price_two_dollars)) // Price = 2.00 USD
	}
}
