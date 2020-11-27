#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_event, decl_module, decl_storage, decl_error, ensure,
    traits::{Get},
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{MultiCurrency};
use orml_utilities::with_transaction_result;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::DispatchError;
use sp_std::{result, prelude::Vec};
#[cfg(test)]
mod tests;

pub trait Trait: system::Trait {
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

    /// The `MultiCurrency` implementation for wrapped.
    type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

    /// Wrapped currency IDs.
    type WrappedCurrencyIds: Get<Vec<CurrencyId>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MinterestProtocol {
	}
}

decl_event!(
	pub enum Event {
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
        /// The currency is not enabled in wrapped protocol.
		NotValidWrappedCurrencyId,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		const WrappedCurrencyIds: Vec<CurrencyId> = T::WrappedCurrencyIds::get();

		/// Mint wrapped tokens.
        #[weight = 10_000]
        fn mint(origin,
            currency_id: CurrencyId,
            #[compact] amount: Balance
        ) {
            with_transaction_result(|| {
                let who = ensure_signed(origin)?;
                let _ = Self::do_mint(&who, currency_id, amount)?;
                Ok(())
            })?;
        }

        /// Burn wrapped tokens.
        #[weight = 10_000]
        fn burn(origin,
            currency_id: CurrencyId,
            #[compact] amount: Balance
        ) {
            with_transaction_result(|| {
                let who = ensure_signed(origin)?;
                T::MultiCurrency::withdraw(currency_id, &who, amount)?;
                Ok(())
            })?;
        }
	}
}

type BalanceResult = result::Result<Balance, DispatchError>;

impl<T: Trait> Module<T> {
    fn do_mint(
        who: &T::AccountId,
        currency_id: CurrencyId,
        amount: Balance,
    ) -> BalanceResult {
        ensure!(
			T::WrappedCurrencyIds::get().contains(&currency_id),
			Error::<T>::NotValidWrappedCurrencyId
		);
        T::MultiCurrency::deposit(currency_id, &who, amount)?;
        Ok(amount)
    }
}
