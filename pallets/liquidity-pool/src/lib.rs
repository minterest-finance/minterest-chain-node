#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
};

use orml_traits::MultiCurrency;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::DispatchResult;
use sp_std::prelude::*;
use pallet_traits::LiquidityPools;


pub const DEFAULT_BALANCE: Balance = 0;

pub trait Trait: frame_system::Trait {
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

    type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_event! (
    pub enum Event {}
);

decl_error! {
    pub enum Error for Module<T: Trait> {

    /// Not enough balance to withdraw.
		NotEnoughBalance,

    /// Liquidity amount overflows maximum.
    /// Only happened when the liquidity currency went wrong and liquidity amount overflows the integer type.
        LiquidityOverflow,

	/// Pool not found.
		PoolNotFound,
	}
}

decl_storage! {
     trait Store for Module<T: Trait> as LiquidityPoolsStorage {
        pub Pools get(fn pools) config(): map hasher(blake2_128_concat) CurrencyId => Balance;
	}
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> LiquidityPools for Module<T> {

    fn add_liquidity(currency_id: &CurrencyId, amount: &Balance) -> DispatchResult {
        Self::do_increase_liquidity(currency_id, amount)
    }

    fn withdraw_liquidity(currency_id: &CurrencyId, amount: &Balance) -> DispatchResult {
        Self::do_withdraw_liquidity(currency_id, amount)
    }
}

impl<T: Trait> Module<T> {

    fn do_increase_liquidity(currency_id: &CurrencyId, amount: &Balance ) -> DispatchResult {
        ensure!(Self::pool_exists(&currency_id), Error::<T>::PoolNotFound);

        let pool_balance = <Pools>::get(*currency_id);
        let new_balance =  pool_balance.checked_add(*amount).ok_or(Error::<T>::LiquidityOverflow)?;

        <Pools>::insert(&currency_id, new_balance);
        Ok(())
    }

    fn do_withdraw_liquidity(currency_id: &CurrencyId, amount: &Balance ) -> DispatchResult {
        ensure!(Self::pool_exists(&currency_id), Error::<T>::PoolNotFound);

        let pool_balance = <Pools>::get(*currency_id);
        let new_balance = pool_balance.checked_sub(*amount).ok_or(Error::<T>::NotEnoughBalance)?;

        <Pools>::insert(&currency_id, new_balance);
        Ok(())
    }

    /// Check if pool exists
    fn pool_exists(currency_id: &CurrencyId) -> bool {
        <Pools>::contains_key(&currency_id)
    }
}
