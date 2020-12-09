#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
};
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::{
    DispatchResult, Permill, RuntimeDebug,
    traits::Zero
};

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

decl_event! (
    pub enum Event {}
);

decl_error! {
    pub enum Error for Module<T: Trait> {}
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

    fn update_reserve_interest_rate(liquidity_added: Balance, liquidity_taken: Balance, currency_id: CurrencyId) -> DispatchResult {
        let current_reserve_balance = Self::reserves(currency_id).total_balance;

        let new_reserve_balance: Balance;

        if liquidity_added != Balance::zero() {
            new_reserve_balance = current_reserve_balance.checked_add(liquidity_added).ok_or("Overflow balance")?;
        } else {
            new_reserve_balance = current_reserve_balance.checked_sub(liquidity_taken).ok_or("Not enough balance")?;
        }

        Reserves::mutate(currency_id, |r| r.total_balance = new_reserve_balance );

        Self::calculate_interest_rate(new_reserve_balance, currency_id)?;

        Ok(())
    }

    fn calculate_interest_rate(_current_reserve_balance: Balance, currency_id: CurrencyId) -> DispatchResult {
        // TODO: some another logic here......
        Reserves::mutate(currency_id, |r| r.current_liquidity_rate = Permill::one());

        Ok(())
    }

}
