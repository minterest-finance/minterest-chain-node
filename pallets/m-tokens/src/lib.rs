#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_event, decl_storage, decl_module
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{MultiReservableCurrency, MultiCurrency};
use orml_utilities::with_transaction_result;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::{
    traits::{StaticLookup},
};

#[cfg(test)]
mod tests;

pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type Currency: MultiReservableCurrency<Self::AccountId>;

    /// The `MultiCurrency` implementation for wrapped.
    type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

}

decl_event! {
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
		/// Approval is made. [owner, spender, amount]
		Approval(AccountId, AccountId, Balance),
    }
}

type BalanceOf<T> = <<T as Trait>::Currency as MultiCurrency<<T as system::Trait>::AccountId>>::Balance;

decl_storage! {
    trait Store for Module<T: Trait> as MTokens {
        //FIXME разобраться как получить баланс конкретной валюты.
        pub BalanceCurrency get(fn balance_of): double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) CurrencyId => BalanceOf<T>;

        // TODO TotalSupply

        // TODO Allowance
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		/// Transfer some balance to another account.
		///
		/// The dispatch origin for this call must be `Signed` by the transactor.
		#[weight = 10_000]
        pub fn transfer(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyId,
			#[compact] amount: Balance,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			T::MultiCurrency::transfer(currency_id, &from, &to, amount)?;
		}

        /// Mint wrapped tokens.
        #[weight = 10_000]
        fn mint(origin,
            to: T::AccountId,
            currency_id: CurrencyId,
            #[compact] amount: Balance
        ) {
            with_transaction_result(|| {
                let _ = ensure_signed(origin)?;
                T::MultiCurrency::deposit(currency_id, &to, amount)?;
                Ok(())
            })?;
        }

        /// Burn wrapped tokens.
        #[weight = 10_000]
        fn burn(origin,
            from: T::AccountId,
            currency_id: CurrencyId,
            #[compact] amount: Balance
        ) {
            with_transaction_result(|| {
                let _ = ensure_signed(origin)?;
                T::MultiCurrency::withdraw(currency_id, &from, amount)?;
                Ok(())
            })?;
        }

        #[weight = 10_000]
        fn approve(origin,
            spender: <T::Lookup as StaticLookup>::Source,
            currency_id: CurrencyId,
            #[compact] value: Balance
        ) {
            with_transaction_result(|| {
                let sender = ensure_signed(origin)?;
                let spender = T::Lookup::lookup(spender)?;

                //TODO
                //<Allowance<T>>::insert((token_id, sender.clone(), spender.clone()), value);

                Self::deposit_event(RawEvent::Approval(sender, spender, value));
                Ok(())
            })?
        }

        #[weight = 10_000]
        fn transfer_from(origin,
            from: T::AccountId,
            to: T::AccountId,
            currency_id: CurrencyId,
            #[compact] value: Balance
        ) {
            with_transaction_result(|| {
                let _sender = ensure_signed(origin)?;
                //TODO
                //let allowance = Self::allowance_of((token_id, from.clone(), sender.clone()));
                //let updated_allowance = allowance.checked_sub(&value).ok_or("Underflow in calculating allowance")?;
                //Self::make_transfer(token_id, from.clone(), to.clone(), value)?;
                //<Allowance<T>>::insert((token_id, from, sender), updated_allowance);
                Ok(())
            })?
        }

    }
}

impl<T: Trait> Module<T> {

}
