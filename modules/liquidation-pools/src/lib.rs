//! # Liquidation Pools Module
//!
//! ## Overview
//!
//! Liquidation Pools are responsible for holding funds for automatic liquidation.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{ensure, pallet_prelude::*, traits::Get};
use frame_system::{ensure_signed, pallet_prelude::*};
use minterest_primitives::{Balance, CurrencyId};
use orml_traits::MultiCurrency;
use pallet_traits::PoolsManager;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{ModuleId, RuntimeDebug};

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Liquidation Pool metadata
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidationPool<BlockNumber> {
	/// Block number that pool was last balancing attempted at.
	pub timestamp: BlockNumber,
	/// Balancing pool frequency.
	pub balancing_period: u32,
}

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Accounts<T> = accounts::Module<T>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + liquidity_pools::Config + accounts::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		/// The Liquidation Pool's module id, keep all assets in Pools.
		type LiquidationPoolsModuleId: Get<ModuleId>;

		#[pallet::constant]
		/// The Liquidation Pool's account id, keep all assets in Pools.
		type LiquidationPoolAccountId: Get<Self::AccountId>;

		/// The `MultiCurrency` implementation.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The dispatch origin of this call must be Administrator.
		RequireAdmin,
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		///  Balancing period has been successfully changed: \[who, new_period\]
		BalancingPeriodChanged(T::AccountId, u32),
	}

	#[pallet::storage]
	#[pallet::getter(fn liquidation_pools)]
	pub(crate) type LiquidationPools<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, LiquidationPool<T::BlockNumber>, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn set_balancing_period(
			origin: OriginFor<T>,
			pool_id: CurrencyId,
			new_period: u32,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			// Write new value into storage.
			LiquidationPools::<T>::mutate(pool_id, |x| x.balancing_period = new_period);

			Self::deposit_event(Event::BalancingPeriodChanged(sender, new_period));

			Ok(().into())
		}
	}
}

impl<T: Config> PoolsManager<T::AccountId> for Pallet<T> {
	/// Gets module account id.
	fn pools_account_id() -> T::AccountId {
		T::LiquidationPoolsModuleId::get().into_account()
	}

	/// Gets current the total amount of cash the liquidation pool has.
	fn get_pool_available_liquidity(pool_id: CurrencyId) -> Balance {
		let module_account_id = Self::pools_account_id();
		<T as Config>::MultiCurrency::free_balance(pool_id, &module_account_id)
	}

	/// Check if pool exists
	fn pool_exists(underlying_asset_id: &CurrencyId) -> bool {
		LiquidationPools::<T>::contains_key(underlying_asset_id)
	}
}
