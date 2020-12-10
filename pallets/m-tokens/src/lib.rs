#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_event, decl_storage, decl_module, decl_error, ensure,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{MultiCurrency};
use orml_utilities::with_transaction_result;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::{
    traits::{StaticLookup},
};
use frame_support::dispatch::DispatchResult;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The `MultiCurrency` implementation for wrapped.
    type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_event! {
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
		/// Approval is made. [currency_id, owner, spender, amount]
		Approval(CurrencyId, AccountId, AccountId, Balance),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as MTokens {
        /// Allowance for an account and token
        Allowance get(fn allowance): map hasher(blake2_128_concat) (CurrencyId, T::AccountId, T::AccountId) => Balance;
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Overflow in calculating allowance
        OverflowAllowance,

        /// Allowance does not exist
        AllowanceDoesNotExist,

        /// Not enough allowance
        NotEnoughAllowance,

        /// Underflow in calculating allowance
        UnderflowAllowance,
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
	    type Error = Error<T>;
		fn deposit_event() = default;

        #[weight = 10_000]
        fn approve(origin,
            spender: <T::Lookup as StaticLookup>::Source,
            currency_id: CurrencyId,
            #[compact] value: Balance
        ) {
            with_transaction_result(|| {
                let sender = ensure_signed(origin)?;
                let spender = T::Lookup::lookup(spender)?;

                let allowance = Self::allowance((currency_id, sender.clone(), spender.clone()));
                let updated_allowance = allowance.checked_add(value)
                    .ok_or(Error::<T>::OverflowAllowance)?;
                <Allowance<T>>::insert((currency_id, sender.clone(), spender.clone()), updated_allowance);

                Self::deposit_event(RawEvent::Approval(currency_id, sender.clone(), spender.clone(), value));
                Ok(())
            })?
        }

        #[weight = 10_000]
        fn transfer_from(_origin,
            from: T::AccountId,
            to: T::AccountId,
            currency_id: CurrencyId,
            #[compact] value: Balance
        ) {
            with_transaction_result(|| {
                ensure!(
                    <Allowance<T>>::contains_key((currency_id, from.clone(), to.clone())),
                    Error::<T>::AllowanceDoesNotExist
                 );
                let allowance = Self::allowance((currency_id, from.clone(), to.clone()));
                ensure!(
                    allowance >= value,
                    Error::<T>::NotEnoughAllowance
                );

                let updated_allowance = allowance.checked_sub(value)
                    .ok_or(Error::<T>::UnderflowAllowance)?;

                T::MultiCurrency::transfer(currency_id, &from, &to, value)?;
                <Allowance<T>>::insert((currency_id, from.clone(), to.clone()), updated_allowance);

                Self::deposit_event(RawEvent::Approval(currency_id, from.clone(), to.clone(), value));
                Ok(())
            })?
        }

    }
}

impl<T: Trait> Module<T> {
    pub fn free_balance(currency_id: CurrencyId, who: &T::AccountId) -> Balance {
        T::MultiCurrency::free_balance(currency_id, &who)
    }

    pub fn deposit(
        currency_id: CurrencyId,
        who: &T::AccountId,
        amount: Balance
    ) -> DispatchResult {
        T::MultiCurrency::deposit(currency_id, &who, amount)?;
        Ok(())
    }

    pub fn withdraw(
        currency_id: CurrencyId,
        who: &T::AccountId,
        amount: Balance
    ) -> DispatchResult {
        T::MultiCurrency::withdraw(currency_id, &who, amount)?;
        Ok(())
    }
}
