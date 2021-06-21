//! # Whitelist Module
//!
//! ## Overview
//!
//! TODO
//!
//! ### Vesting Schedule
//!
//! TODO

//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! TODO

#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::pallet_prelude::*;
use frame_support::traits::Contains;
pub use module::*;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use frame_system::pallet_prelude::OriginFor;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The origin which may umanage members in whitelist. Root or
		/// Half Minterest Council can always do this.
		type WhitelistOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		/// A maximum number of members. When membership reaches this number, no new members may
		/// join.
		type MaxMembers: Get<u8>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The member cannot be added to the whitelist because it has already been added.
		AlreadyMember,
		/// The member cannot be removed from the whitelist because it is not a member.
		NotMember,
		/// Cannot add another member because the limit is already reached.
		MembershipLimitReached,
		/// Cannot remove a member because at least one member must remain.
		MustBeAtLeastOneMember,
		/// The value does not exists or it fails to decode the length.
		FailsDecodeLength,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// The given member was added to the whitelist: \[who\]
		MemberAdded(T::AccountId),
		/// The given member was removed from the whitelist: \[who\]
		MemberRemoved(T::AccountId),
	}

	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub(crate) type Members<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub members: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { members: vec![] }
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

			let mut members = self.members.clone();
			members.sort();
			Members::<T>::put(members)
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
		#[pallet::weight(0)]
		pub fn add_member(origin: OriginFor<T>, new_account: T::AccountId) -> DispatchResultWithPostInfo {
			T::WhitelistOrigin::ensure_origin(origin)?;
			ensure!(
				Members::<T>::decode_len().ok_or(Error::<T>::FailsDecodeLength)? < T::MaxMembers::get() as usize,
				Error::<T>::MembershipLimitReached
			);

			let mut members = Self::members();
			let location = members
				.binary_search(&new_account)
				.err()
				.ok_or(Error::<T>::AlreadyMember)?;
			members.insert(location, new_account.clone());
			Members::<T>::put(&members);

			Self::deposit_event(Event::MemberAdded(new_account));
			Ok(().into())
		}

		/// Remove a member from the whitelist.
		///
		/// - `who`: the account that is being removed from the whitelist.
		///
		/// The dispatch origin of this call must be 'WhitelistOrigin'.
		#[pallet::weight(0)]
		pub fn remove_member(origin: OriginFor<T>, who: T::AccountId) -> DispatchResultWithPostInfo {
			T::WhitelistOrigin::ensure_origin(origin)?;

			ensure!(
				Members::<T>::decode_len().ok_or(Error::<T>::FailsDecodeLength)? > 1,
				Error::<T>::MustBeAtLeastOneMember
			);

			let mut members = Self::members();
			let location = members.binary_search(&who).ok().ok_or(Error::<T>::NotMember)?;
			members.remove(location);
			Members::<T>::put(&members);

			Self::deposit_event(Event::MemberRemoved(who));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {}

impl<T: Config> Contains<T::AccountId> for Module<T> {
	fn contains(t: &T::AccountId) -> bool {
		Self::members().binary_search(t).is_ok()
	}

	fn sorted_members() -> Vec<T::AccountId> {
		Self::members()
	}

	fn count() -> usize {
		Members::<T>::decode_len().unwrap_or(0)
	}
}
