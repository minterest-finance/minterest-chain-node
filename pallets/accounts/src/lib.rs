#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use frame_system::{self as system, ensure_root};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// A maximum number of members. When membership reaches this number, no new members may join.
pub const MAX_MEMBERS: u32 = 16;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Accounts {
		AllowedAccounts get(fn accounts): map hasher(blake2_128_concat) T::AccountId => ();
		MemberCount: u32;
	}
}

decl_event!(
	pub enum Event<T>
	where
		AccountId = <T as system::Trait>::AccountId,
	{
		/// New account is added to the allow-list: \[who\]
		AccountAdded(AccountId),
		/// Account is removed from the allow-list: \[who\]
		AccountRemoved(AccountId),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// The account cannot be added to the allowed list because it has already been added.
		AlreadyMember,
		/// The account cannot be removed from the allowed list because it is not a member
		NotMember,
		/// Cannot add another member because the limit is already reached.
		MembershipLimitReached,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Adds a new account to the allow-list.
		///
		/// The dispatch origin of this call must be _Root_.
		#[weight = 0]
		fn add_member(origin, new_account: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			let member_count = MemberCount::get();
			ensure!(member_count < MAX_MEMBERS, Error::<T>::MembershipLimitReached);

			ensure!(!AllowedAccounts::<T>::contains_key(&new_account), Error::<T>::AlreadyMember);

			AllowedAccounts::<T>::insert(&new_account, ());
			MemberCount::put(member_count + 1);
			Self::deposit_event(RawEvent::AccountAdded(new_account));
			Ok(())
		}

		/// Remove an account from the allow-list.
		///
		/// The dispatch origin of this call must be _Root_.
		#[weight = 0]
		fn remove_member(origin, account_to_remove: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(AllowedAccounts::<T>::contains_key(&account_to_remove), Error::<T>::NotMember);

			AllowedAccounts::<T>::remove(&account_to_remove);
			MemberCount::mutate(|v| *v -= 1);
			Self::deposit_event(RawEvent::AccountRemoved(account_to_remove));
			Ok(())
		}
	}
}

impl<T: Trait> Module<T> {}
