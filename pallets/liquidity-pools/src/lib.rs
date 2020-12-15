#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use minterest_primitives::{Balance, CurrencyId};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Zero, DispatchResult, Permill, RuntimeDebug};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Reserve {
	pub total_balance: Balance,
	pub current_liquidity_rate: Permill,
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event!(
	pub enum Event {}
);

decl_error! {
	pub enum Error for Module<T: Trait> {

	PoolNotFound,

	NotEnoughBalance,

	BalanceOverflowed,
	}
}

decl_storage! {
	 trait Store for Module<T: Trait> as LiquidityPoolsStorage {
		pub Reserves get(fn reserves) config(): map hasher(blake2_128_concat) CurrencyId => Reserve;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> Module<T> {
	pub fn update_state_on_deposit(amount: Balance, currency_id: CurrencyId) -> DispatchResult {
		Self::update_reserve_interest_rate(amount, Balance::zero(), currency_id)?;

		Ok(())
	}

	pub fn update_state_on_redeem(amount: Balance, currency_id: CurrencyId) -> DispatchResult {
		Self::update_reserve_interest_rate(Balance::zero(), amount, currency_id)?;

		Ok(())
	}

	pub fn get_reserve_available_liquidity(currency_id: CurrencyId) -> Balance {
		Self::reserves(currency_id).total_balance
	}

	fn update_reserve_interest_rate(
		liquidity_added: Balance,
		liquidity_taken: Balance,
		underlying_asset_id: CurrencyId,
	) -> DispatchResult {
		ensure!(Self::pool_exists(&underlying_asset_id), Error::<T>::PoolNotFound);

		let current_reserve_balance = Self::reserves(underlying_asset_id).total_balance;

		let new_reserve_balance: Balance;

		if liquidity_added != Balance::zero() {
			new_reserve_balance = current_reserve_balance
				.checked_add(liquidity_added)
				.ok_or(Error::<T>::BalanceOverflowed)?;
		} else {
			new_reserve_balance = current_reserve_balance
				.checked_sub(liquidity_taken)
				.ok_or(Error::<T>::NotEnoughBalance)?;
		}

		Reserves::mutate(underlying_asset_id, |r| r.total_balance = new_reserve_balance);

		Ok(())
	}

	pub fn set_current_liquidity_rate(underlying_asset_id: CurrencyId, _rate: Permill) -> DispatchResult {
		Reserves::mutate(underlying_asset_id, |r| {
			r.current_liquidity_rate = Permill::from_percent(44)
		});
		Ok(())
	}

	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		Reserves::contains_key(underlying_asset_id)
	}
}
