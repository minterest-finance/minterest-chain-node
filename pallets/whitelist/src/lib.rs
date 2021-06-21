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
		/// The member cannot be added to the whitelist because it has already been added.
		AlreadyMember,
		/// The member cannot be removed from the whitelist because it is not a member.
		NotMember,
		/// Cannot add another member because the limit is already reached.
		MembershipLimitReached,
		/// Cannot remove a member because at least one member must remain.
		MustBeAtLeastOneMember,
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
				"duplicate member account in whitelist in genesis."
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
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {}
