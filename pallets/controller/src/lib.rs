#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system::{self as system};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use sp_runtime::{DispatchError, DispatchResult};

use sp_std::result;

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

type LiquidityPools<T> = liquidity_pools::Module<T>;

pub trait Trait: liquidity_pools::Trait {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The `MultiCurrency` implementation for wrapped.
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_event! {
	pub enum Event {}
}

decl_storage! {
	trait Store for Module<T: Trait> as X {

	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
	InvalidValues,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;


	}
}

type RateResult = result::Result<Rate, DispatchError>;

impl<T: Trait> Module<T> {
	pub fn get_exchange_rate(underlying_asset_id: CurrencyId) -> RateResult {
		// The total amount of cash the market has
		let _total_cash = <LiquidityPools<T>>::get_reserve_available_liquidity(underlying_asset_id);

		// Total number of tokens in circulation
		let _total_supply = T::MultiCurrency::total_issuance(underlying_asset_id);

		// Self::caclulate_exchange_rate(total_cash, total_supply)?;

		Ok(Rate::from_inner(1))
	}

	pub fn calculate_user_global_data(_who: T::AccountId) -> DispatchResult {
		Ok(())
	}

	pub fn calculate_total_available_collateral(_amount: Balance, _underlying_asset_id: CurrencyId) -> DispatchResult {
		Ok(())
	}

	pub fn calculate_liquidity_rate(_underlying_asset_id: CurrencyId) -> RateResult {
		Ok(Rate::from_inner(1))
	}

	// fn caclulate_exchange_rate(total_cash: Balance, total_supply: Balance) -> RateResult {
	// 	let rate = total_cash.checked_div(total_supply).ok_or(Error::<T>::InvalidValues)?;
	//     let rates = Permill::from_percent();
	// }
}
