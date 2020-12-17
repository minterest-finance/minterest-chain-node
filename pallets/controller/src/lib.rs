#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system::{self as system};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use sp_runtime::{traits::CheckedDiv, DispatchError, DispatchResult, FixedPointNumber};

use sp_runtime::traits::CheckedMul;
use sp_std::result;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

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

		/// Number overflow in calculation.
		NumOverflow,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

	}
}

type RateResult = result::Result<Rate, DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

impl<T: Trait> Module<T> {
	pub fn get_exchange_rate(currency_id: CurrencyId) -> RateResult {
		// The total amount of cash the market has
		let _total_cash = <LiquidityPools<T>>::get_reserve_available_liquidity(currency_id);

		// Total number of tokens in circulation
		let _total_supply = T::MultiCurrency::total_issuance(currency_id);

		// Self::caclulate_exchange_rate(total_cash, total_supply)?;

		Ok(Rate::saturating_from_rational(8, 10)) // 80%
	}

	pub fn convert_to_wrapped(underlying_asset_id: CurrencyId, underlying_amount: Balance) -> BalanceResult {
		let exchange_rate = Self::get_exchange_rate(underlying_asset_id)?;

		let wrapped_amount = Rate::from_inner(underlying_amount)
			.checked_div(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(wrapped_amount)
	}

	pub fn convert_from_wrapped(wrapped_id: CurrencyId, wrapped_amount: Balance) -> BalanceResult {
		let exchange_rate = Self::get_exchange_rate(wrapped_id)?;

		let underlying_amount = Rate::from_inner(wrapped_amount)
			.checked_mul(&exchange_rate)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Ok(underlying_amount)
	}

	pub fn calculate_user_global_data(_who: T::AccountId) -> DispatchResult {
		//FIXME
		let _price_from_oracle = 1;
		Ok(())
	}

	pub fn calculate_total_available_collateral(_amount: Balance, _underlying_asset_id: CurrencyId) -> DispatchResult {
		//FIXME
		let _price_from_oracle = 1;
		Ok(())
	}

	pub fn accrue_interest_rate() -> DispatchResult {
		//FIXME Applies accrued interest to total borrows and reserves.
		// This calculates interest accrued from the last checkpointed block up to the current block
		// and writes new checkpoint to storage.
		Ok(())
	}

	pub fn calculate_interest_rate(_underlying_asset_id: CurrencyId) -> RateResult {
		//FIXME
		Ok(Rate::from_inner(1))
	}

	// fn caclulate_exchange_rate(total_cash: Balance, total_supply: Balance) -> RateResult {
	// 	let rate = total_cash.checked_div(total_supply).ok_or(Error::<T>::InvalidValues)?;
	//     let rates = Permill::from_percent();
	// }
}
