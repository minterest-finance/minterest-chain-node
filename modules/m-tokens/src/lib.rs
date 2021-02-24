#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::{ensure_signed, pallet_prelude::*};
use minterest_primitives::{Balance, CurrencyId};
use orml_traits::MultiCurrency;
use sp_runtime::traits::StaticLookup;

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The `MultiCurrency` implementation for wrapped.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Overflow in calculating allowance.
		OverflowAllowance,
		/// Allowance does not exist.
		AllowanceDoesNotExist,
		/// Not enough allowance.
		NotEnoughAllowance,
		/// Underflow in calculating allowance.
		UnderflowAllowance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Approval is made. [currency_id, owner, spender, amount]
		Approval(CurrencyId, T::AccountId, T::AccountId, Balance),
	}

	/// Allowance for an account and token.
	#[pallet::storage]
	#[pallet::getter(fn allowance)]
	type Allowance<T: Config> =
		StorageMap<_, Twox64Concat, (CurrencyId, T::AccountId, T::AccountId), Balance, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Allows `spender` to withdraw from the caller's account multiple times, up to
		/// the `value` amount.
		///
		/// If this function is called again it overwrites the current allowance with `value`.
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn approve(
			origin: OriginFor<T>,
			spender: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyId,
			value: Balance,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			let spender = T::Lookup::lookup(spender)?;

			let allowance = Self::allowance((currency_id, sender.clone(), spender.clone()));
			let updated_allowance = allowance.checked_add(value).ok_or(Error::<T>::OverflowAllowance)?;
			<Allowance<T>>::insert((currency_id, sender.clone(), spender.clone()), updated_allowance);

			Self::deposit_event(Event::Approval(currency_id, sender, spender, value));
			Ok(().into())
		}

		/// Transfers `value` tokens on the behalf of `from` to the account `to`.
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn transfer_from(
			_origin: OriginFor<T>,
			from: T::AccountId,
			to: T::AccountId,
			currency_id: CurrencyId,
			value: Balance,
		) -> DispatchResultWithPostInfo {
			ensure!(
				<Allowance<T>>::contains_key((currency_id, from.clone(), to.clone())),
				Error::<T>::AllowanceDoesNotExist
			);
			let allowance = Self::allowance((currency_id, from.clone(), to.clone()));
			ensure!(allowance >= value, Error::<T>::NotEnoughAllowance);

			let updated_allowance = allowance.checked_sub(value).ok_or(Error::<T>::UnderflowAllowance)?;

			T::MultiCurrency::transfer(currency_id, &from, &to, value)?;
			<Allowance<T>>::insert((currency_id, from.clone(), to.clone()), updated_allowance);

			Self::deposit_event(Event::Approval(currency_id, from.clone(), to.clone(), value));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {}
