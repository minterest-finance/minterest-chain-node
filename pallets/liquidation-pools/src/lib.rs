#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::ModuleId;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	/// The Liquidation Pool's module id, keep all assets in Pools.
	type ModuleId: Get<ModuleId>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Exchange {
	}
}

decl_event!(
	pub enum Event {}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// The Liquidity Pool's module id, keep all assets in Pools.
		const ModuleId: ModuleId = T::ModuleId::get();
	}
}

impl<T: Trait> Module<T> {
	/// Gets module account id.
	pub fn pools_account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
}
