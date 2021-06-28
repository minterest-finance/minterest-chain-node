//! # Whitelist Module
//!
//! ## Overview
//!
//! Whitelist module provides the necessary functionality for the protocol to work in whitelist
//! mode. Allows control of membership of a set of `AccountID`s, useful for managing
//! membership of a whitelist. There can be no more than `MaxMembers` in the whitelist at the same
//! time, and there must always be at least one user in the whitelist.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `add_member` - Add a new member to the whitelist. Root or half Minterest Council can
//! always do this.
//! - `remove_member` - Remove a member from the whitelist. Root or half Minterest Council
//! can always do this.
//! - `switch_whitelist_mode` - Enable / disable whitelist mode.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional, IterableStorageMap};
use frame_system::pallet_prelude::OriginFor;
pub use module::*;
use pallet_traits::WhitelistManager;
use sp_std::collections::btree_set::BTreeSet;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The origin which may manage members in whitelist. Root or
		/// Half Minterest Council can always do this.
		type WhitelistOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		/// A maximum number of members. When membership reaches this number, no new members may
		/// join.
		type MaxMembers: Get<u8>;

		/// Weight information for the extrinsics.
		type WhitelistWeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The member cannot be added to the whitelist because it has already been added.
		MemberAlreadyAdded,
		/// The member cannot be removed from the whitelist because it is not a member.
		MemberNotExist,
		/// Cannot add another member because the limit is already reached.
		MembershipLimitReached,
		/// Cannot remove a member because at least one member must remain.
		MustBeAtLeastOneMember,
		/// Error changing the protocol mode. The mode you want to set is already in effect.
		ModeChangeError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// The given member was added to the whitelist: \[who\]
		MemberAdded(T::AccountId),
		/// The given member was removed from the whitelist: \[who\]
		MemberRemoved(T::AccountId),
		/// Protocol operation mode switched: \[is_whitelist_mode\]
		ProtocolOperationModeSwitched(bool),
	}

	/// The set of all members.
	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, (), OptionQuery>;

	/// The total number of members stored in the map.
	/// Because the map does not store its size, we must store it separately.
	#[pallet::storage]
	#[pallet::getter(fn member_count)]
	pub type MemberCount<T: Config> = StorageValue<_, u8, ValueQuery>;

	/// Boolean variable. Protocol operation mode. In whitelist mode, only members
	/// from whitelist can work with protocol.
	#[pallet::storage]
	#[pallet::getter(fn whitelist_mode)]
	pub(crate) type WhitelistMode<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub members: Vec<T::AccountId>,
		pub whitelist_mode: bool,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				members: vec![],
				whitelist_mode: false,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// ensure no duplicates exist.
			let unique_whitelist_members = self.members.iter().cloned().collect::<std::collections::BTreeSet<_>>();
			assert!(
				unique_whitelist_members.len() == self.members.len(),
				"Duplicate member account in whitelist in genesis."
			);

			assert!(
				self.members.len() <= T::MaxMembers::get() as usize,
				"Exceeded the number of whitelist members in genesis."
			);

			self.members.iter().for_each(|who| {
				Members::<T>::insert(who, ());
			});

			MemberCount::<T>::put(self.members.len() as u8);
			WhitelistMode::<T>::put(self.whitelist_mode);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a new member to the whitelist.
		///
		/// - `new_account`: the account that is being added to the whitelist.
		///
		/// The dispatch origin of this call must be 'WhitelistOrigin'.
		#[pallet::weight(T::WhitelistWeightInfo::add_member((<T as Config>::MaxMembers::get() / 2) as u32))]
		pub fn add_member(origin: OriginFor<T>, new_account: T::AccountId) -> DispatchResultWithPostInfo {
			T::WhitelistOrigin::ensure_origin(origin)?;
			let member_count = MemberCount::<T>::get();

			ensure!(member_count < T::MaxMembers::get(), Error::<T>::MembershipLimitReached);
			ensure!(!Self::is_whitelist_member(&new_account), Error::<T>::MemberAlreadyAdded);

			Members::<T>::insert(&new_account, ());
			MemberCount::<T>::put(member_count + 1);
			Self::deposit_event(Event::MemberAdded(new_account));
			Ok(().into())
		}

		/// Remove a member from the whitelist.
		///
		/// - `who`: the account that is being removed from the whitelist.
		///
		/// The dispatch origin of this call must be 'WhitelistOrigin'.
		#[pallet::weight(T::WhitelistWeightInfo::remove_member((<T as Config>::MaxMembers::get() / 2) as u32))]
		pub fn remove_member(origin: OriginFor<T>, account_to_remove: T::AccountId) -> DispatchResultWithPostInfo {
			T::WhitelistOrigin::ensure_origin(origin)?;

			ensure!(
				Self::is_whitelist_member(&account_to_remove),
				Error::<T>::MemberNotExist
			);

			ensure!(MemberCount::<T>::get() > 1, Error::<T>::MustBeAtLeastOneMember);

			Members::<T>::remove(&account_to_remove);
			MemberCount::<T>::mutate(|v| *v -= 1);
			Self::deposit_event(Event::MemberRemoved(account_to_remove));
			Ok(().into())
		}

		/// Enable / disable whitelist mode.
		///
		/// The dispatch origin of this call must be 'WhitelistOrigin'.
		#[pallet::weight(T::WhitelistWeightInfo::switch_whitelist_mode())]
		#[transactional]
		pub fn switch_whitelist_mode(origin: OriginFor<T>, new_state: bool) -> DispatchResultWithPostInfo {
			T::WhitelistOrigin::ensure_origin(origin)?;
			WhitelistMode::<T>::try_mutate(|mode| -> DispatchResultWithPostInfo {
				ensure!(*mode != new_state, Error::<T>::ModeChangeError);
				*mode = new_state;
				Self::deposit_event(Event::ProtocolOperationModeSwitched(new_state));
				Ok(().into())
			})
		}
	}
}

impl<T: Config> WhitelistManager<T::AccountId> for Pallet<T> {
	/// Protocol operation mode. In whitelist mode, only members from whitelist can work with
	/// protocol.
	fn is_whitelist_mode_enabled() -> bool {
		WhitelistMode::<T>::get()
	}

	/// Checks if the account is a whitelist member.
	fn is_whitelist_member(who: &T::AccountId) -> bool {
		Members::<T>::contains_key(&who)
	}

	/// Returns the set of all accounts in the whitelist.
	fn whitelist_members() -> BTreeSet<T::AccountId> {
		<Members<T> as IterableStorageMap<T::AccountId, ()>>::iter()
			.map(|(acct, _)| acct)
			.collect::<BTreeSet<_>>()
	}
}
