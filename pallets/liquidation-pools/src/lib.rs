//! # Liquidation Pools Module
//!
//! ## Overview
//!
//! Liquidation Pools are responsible for holding funds for automatic liquidation.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::traits::Get;
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use minterest_primitives::{Balance, CurrencyId};
use orml_traits::MultiCurrency;
use pallet_traits::PoolsManager;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{ModuleId, RuntimeDebug};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Liquidtaion Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct Pool {}

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	/// The Liquidation Pool's module id, keep all assets in Pools.
	type ModuleId: Get<ModuleId>;

	/// The `MultiCurrency` implementation.
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Exchange {
		 /// Liquidation pool information.
		pub LiquidationPools get(fn liquidation_pools) config(): map hasher(blake2_128_concat) CurrencyId => Pool;
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

impl<T: Trait> PoolsManager<T::AccountId> for Module<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	/// Gets current the total amount of cash the liquidation pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		T::MultiCurrency::free_balance(pool_id, &module_account_id)
	}

	/// Check if pool exists
	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		LiquidationPools::contains_key(underlying_asset_id)
	}
}
