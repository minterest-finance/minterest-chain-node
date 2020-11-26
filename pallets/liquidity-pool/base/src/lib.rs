#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    // storage::IterableStorageMap,
    // StorageMap,
    traits::{Currency, EnsureOrigin, Get, ReservableCurrency},
    weights::{DispatchClass, Weight},
};

use frame_system::ensure_signed;
use orml_traits::MultiCurrency;
use orml_utilities::with_transaction_result;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::{
    traits::{AccountIdConversion, One},
    DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::{prelude::*, result};


pub const DEFAULT_BALANCE: Balance = 0;

pub trait Trait: frame_system::Trait {

    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Trait>::AccountId,

    {
            /// Pool has been created;
            LiquidityPoolCreated(CurrencyId, Balance),

		    /// Liquidity has been added;
            LiquidityAdded(CurrencyId, Balance, AccountId),

            /// Liquidity has been withdrawn;
            LiquidityWithdraw(CurrencyId, Balance, AccountId),
    }
}

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
     trait Store for Module<T: Trait> as LiquidityPoolStorage {
        pub Pools get(fn pools): map hasher(blake2_128_concat) CurrencyId => Balance;
	}
	add_extra_genesis {
	    config(pools): Vec<Pools>;
	}
}
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        // Create pool for certain asset.
        #[weight = 10_000]
        pub fn create_pool(origin, currency_id: CurrencyId) {
            with_transaction_result(|| {
                ensure!(!Self::pool_exists(&currency_id), Error::<T>::PoolNotFound);

                let _who = ensure_signed(origin)?;
                Self::do_create_pool(&currency_id)?;
                Self::deposit_event(RawEvent::LiquidityPoolCreated(currency_id, DEFAULT_BALANCE));
                Ok(())
            })?;
        }


        // Add liquidity to the pool
        #[weight = 10_000]
       pub fn add_liquidity(origin, currency_id: CurrencyId, amount: Balance) {
           with_transaction_result(|| {
                ensure!(Self::pool_exists(&currency_id), Error::<T>::PoolNotFound);

                let who = ensure_signed(origin)?;
                Self::do_increase_liquidity(&currency_id, &amount)?;
                Self::deposit_event(RawEvent::LiquidityAdded(currency_id, amount, who));
                Ok(())
           })?;
        }

        // Withdraw liquidity from the pool
        #[weight = 10_000]
       pub fn withdraw_liquidity(origin, currency_id: CurrencyId, amount: Balance) {
		   with_transaction_result(|| {
                ensure!(Self::pool_exists(&currency_id), Error::<T>::PoolNotFound);

                let who = ensure_signed(origin)?;
                Self::do_withdraw_liquidity(&currency_id, &amount);
                Self::deposit_event(RawEvent::LiquidityWithdraw(currency_id, amount, who));
                Ok(())
		   })?;
	    }
    }
}

impl<T: Trait> Module<T> {

    fn do_create_pool(currency_id: &CurrencyId) -> DispatchResult {
        <Pools>::insert(&currency_id, DEFAULT_BALANCE);
        Ok(())
    }

    fn do_increase_liquidity(currency_id: &CurrencyId, amount: &Balance ) -> DispatchResult {
        let pool_balance = <Pools>::get(*currency_id);
        let new_balance =  pool_balance.checked_add(*amount).ok_or(Error::<T>::LiquidityOverflow)?;

        <Pools>::insert(&currency_id, new_balance);
        Ok(())
    }

    fn do_withdraw_liquidity(currency_id: &CurrencyId, amount: &Balance ) -> DispatchResult {
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