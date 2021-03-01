//! # Accounts Module
//!
//! ## Overview
//!
//! TODO: add overview.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{ensure, pallet_prelude::*, transactional, IterableStorageMap};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_std::collections::btree_set::BTreeSet;

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

		#[pallet::constant]
		/// A maximum number of members. When membership reaches this number, no new members may
		/// join.
		type MaxMembers: Get<u8>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The account cannot be added to the allowed list because it has already been added.
		AlreadyMember,
		/// The account cannot be removed from the allowed list because it is not a member.
		NotAnAdmin,
		/// Cannot add another member because the limit is already reached.
		MembershipLimitReached,
		/// Cannot remove a member because ay least one member must remain.
		MustBeAtLeastOneMember,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New account is added to the allow-list: \[who\]
		AccountAdded(T::AccountId),
		/// Account is removed from the allow-list: \[who\]
		AccountRemoved(T::AccountId),
		/// The caller is a member: \[who\]
		IsAnAdmin(T::AccountId),
	}

	#[pallet::storage]
	#[pallet::getter(fn allowed_accounts)]
	pub(crate) type AllowedAccounts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn member_count)]
	type MemberCount<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub allowed_accounts: Vec<(T::AccountId, ())>,
		pub member_count: u8,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				allowed_accounts: vec![],
				member_count: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.allowed_accounts
				.iter()
				.for_each(|(who, _)| AllowedAccounts::<T>::insert(who, ()));
			MemberCount::<T>::put(self.member_count);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Adds a new account to the allow-list.
		///
		/// The dispatch origin of this call must be _Root_.
		#[pallet::weight(0)]
		#[transactional]
		pub fn add_member(origin: OriginFor<T>, new_account: T::AccountId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let member_count = MemberCount::<T>::get();
			ensure!(member_count < T::MaxMembers::get(), Error::<T>::MembershipLimitReached);

			ensure!(
				!AllowedAccounts::<T>::contains_key(&new_account),
				Error::<T>::AlreadyMember
			);

			AllowedAccounts::<T>::insert(&new_account, ());
			MemberCount::<T>::put(member_count + 1);
			Self::deposit_event(Event::AccountAdded(new_account));
			Ok(().into())
		}

		/// Remove an account from the allow-list.
		///
		/// The dispatch origin of this call must be _Root_.
		#[pallet::weight(0)]
		#[transactional]
		pub fn remove_member(origin: OriginFor<T>, account_to_remove: T::AccountId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				AllowedAccounts::<T>::contains_key(&account_to_remove),
				Error::<T>::NotAnAdmin
			);

			let member_count = MemberCount::<T>::get();
			ensure!(member_count > 1, Error::<T>::MustBeAtLeastOneMember);

			AllowedAccounts::<T>::remove(&account_to_remove);
			MemberCount::<T>::mutate(|v| *v -= 1);
			Self::deposit_event(Event::AccountRemoved(account_to_remove));
			Ok(().into())
		}

		/// Checks whether the caller is a member of the allow-list.
		/// Emits an event if they are, and errors if not.
		#[pallet::weight(0)]
		#[transactional]
		pub fn is_admin(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			ensure!(AllowedAccounts::<T>::contains_key(&caller), Error::<T>::NotAnAdmin);
			Self::deposit_event(Event::IsAnAdmin(caller));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Checks whether the caller is a member of the allow-list.
	pub fn is_admin_internal(caller: &T::AccountId) -> bool {
		let members = <AllowedAccounts<T> as IterableStorageMap<T::AccountId, ()>>::iter()
			.map(|(acct, _)| acct)
			.collect::<BTreeSet<_>>();

		members.contains(&caller)
	}
}

// RPC method
impl<T: Config> Pallet<T> {
	pub fn is_admin_rpc(caller: &T::AccountId) -> Option<bool> {
		Some(Self::is_admin_internal(&caller))
	}
}
