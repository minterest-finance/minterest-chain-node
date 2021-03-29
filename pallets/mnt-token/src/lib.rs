//! # MNT token Module
//!
//! Provides functionality for minting MNT tokens.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{LiquidityPoolsManager, PriceProvider};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero},
	DispatchResult, FixedPointNumber, FixedU128,
};
use sp_std::{convert::TryInto, result, vec::Vec};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// TODO MOVE TYPES TO ANOTHER FILE
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MntState<T: Config> {
	pub index: FixedU128,
	pub block_number: T::BlockNumber,
}

impl<T: Config> MntState<T> {
	fn new() -> MntState<T> {
		MntState {
			index: Rate::one(), // initial index
			block_number: frame_system::Module::<T>::block_number(),
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MntPoolState<T: Config> {
	pub supply_state: MntState<T>,
	pub borrow_state: MntState<T>,
}

impl<T: Config> MntPoolState<T> {
	fn new() -> MntPoolState<T> {
		MntPoolState {
			supply_state: MntState::new(),
			borrow_state: MntState::new(),
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct AwardeeData {
	pub index: FixedU128,
	pub acquired_mnt: Rate,
}

impl AwardeeData {
	fn new() -> AwardeeData {
		AwardeeData {
			index: Rate::one(), // initial index
			acquired_mnt: Rate::zero(),
		}
	}
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Provides Liquidity Pool functionality
		type LiquidityPoolsManager: LiquidityPoolsManager;

		/// The origin which may update MNT token parameters. Root can
		/// always do this.
		type UpdateOrigin: EnsureOrigin<Self::Origin>;

		/// The price source of currencies
		type PriceSource: PriceProvider<CurrencyId>;

		/// Enabled underlying asset IDs.
		type EnabledUnderlyingAssetId: Get<Vec<CurrencyId>>;

		/// Enabled currency pairs.
		type EnabledCurrencyPair: Get<Vec<CurrencyPair>>;

		/// The `MultiCurrency` implementation for wrapped.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Trying to enable already enabled minting for pool
		MntMintingAlreadyEnabled,

		/// Trying to disable MNT minting that wasn't enabled
		MntMintingNotEnabled,

		/// Arithmetic calculation overflow
		NumOverflow,

		/// Get underlying currency price is failed
		GetUnderlyingPriceFail,

		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,

		/// Error that never should happen
		StorageIsCorrupted,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Change rate event (old_rate, new_rate)
		NewMntRate(Rate, Rate),

		/// MNT minting enabled for pool
		MntMintingEnabled(CurrencyId),

		/// MNT minting disabled for pool
		MntMintingDisabled(CurrencyId),

		/// Emitted when MNT is distributed to a supplier
		/// (CurrencyId, Reciever, Amount of distributed tokens, supply index)
		MntDistributedToSupplier(CurrencyId, T::AccountId, Rate, Rate),
	}

	/// MNT minting rate per block
	#[pallet::storage]
	#[pallet::getter(fn mnt_rate)]
	type MntRate<T: Config> = StorageValue<_, Rate, ValueQuery>;

	/// MNT minting speed for each pool
	#[pallet::storage]
	#[pallet::getter(fn mnt_speeds)]
	pub(crate) type MntSpeeds<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Rate, OptionQuery>;

	// TODO Description.
	// P.S. Could I merge MntSpeeds and MntPoolsState storage into one?
	#[pallet::storage]
	#[pallet::getter(fn mnt_pools_state)]
	pub(crate) type MntPoolsState<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, MntPoolState<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mnt_supplier_data)]
	pub(crate) type MntSupplierData<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, AwardeeData, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub mnt_rate: Rate,
		pub minted_pools: Vec<CurrencyId>,
		pub _marker: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				mnt_rate: Rate::zero(),
				minted_pools: vec![],
				_marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			MntRate::<T>::put(&self.mnt_rate);
			for currency_id in &self.minted_pools {
				MntSpeeds::<T>::insert(currency_id, Rate::zero());
				MntPoolsState::<T>::insert(currency_id, MntPoolState::new());
			}
			if !self.minted_pools.is_empty() {
				Pallet::<T>::refresh_mnt_speeds().expect("Calculate MntSpeeds is failed");
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		#[transactional]
		/// Enable MNT minting for pool and recalculate MntSpeeds
		pub fn enable_mnt_minting(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::EnabledUnderlyingAssetId::get()
					.into_iter()
					.any(|asset_id| asset_id == currency_id),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				!MntSpeeds::<T>::contains_key(currency_id),
				Error::<T>::MntMintingAlreadyEnabled
			);
			MntSpeeds::<T>::insert(currency_id, Rate::zero());
			MntPoolsState::<T>::insert(currency_id, MntPoolState::new());
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::MntMintingEnabled(currency_id));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		/// Disable MNT minting for pool and recalculate MntSpeeds
		pub fn disable_mnt_minting(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				T::EnabledUnderlyingAssetId::get()
					.into_iter()
					.any(|asset_id| asset_id == currency_id),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				MntSpeeds::<T>::contains_key(currency_id),
				Error::<T>::MntMintingNotEnabled
			);
			MntSpeeds::<T>::remove(currency_id);
			MntPoolsState::<T>::remove(currency_id);
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::MntMintingDisabled(currency_id));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		/// Set MNT rate and recalculate MntSpeeds distribution
		pub fn set_mnt_rate(origin: OriginFor<T>, new_rate: Rate) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let old_rate = MntRate::<T>::get();
			MntRate::<T>::put(new_rate);
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::NewMntRate(old_rate, new_rate));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	// TODO description
	// TODO(2) Add deposit_event MntDistributedToSupplier, after calling this function
	fn distribute_supplier_mnt(underlying_id: CurrencyId, supplier: &T::AccountId) -> DispatchResult {
		// delta_index = mnt_supply_index - mnt_supplier_index
		// supplier_delta = supplier_mtoken_balance * delta_index
		// supplier_mnt_balance += supplier_delta
		// mnt_supplier_index = mnt_supply_index
		let supply_index = MntPoolsState::<T>::get(underlying_id)
			.ok_or(Error::<T>::StorageIsCorrupted)?
			.supply_state
			.index;

		let mut supplier_data = MntSupplierData::<T>::get(supplier)
			.or(Some(AwardeeData::new()))
			.unwrap();

		let delta_index = supply_index
			.checked_sub(&supplier_data.index)
			.ok_or(Error::<T>::NumOverflow)?;

		// TODO rework this
		let wrapped_id = T::EnabledCurrencyPair::get()
			.iter()
			.find(|currency_pair| currency_pair.underlying_id == underlying_id)
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?
			.wrapped_id;
		let supplier_balance = Rate::saturating_from_integer(T::MultiCurrency::total_balance(wrapped_id, supplier));

		let supplier_delta = delta_index
			.checked_mul(&supplier_balance)
			.ok_or(Error::<T>::NumOverflow)?;

		supplier_data.acquired_mnt = supplier_data
			.acquired_mnt
			.checked_add(&supplier_delta)
			.ok_or(Error::<T>::NumOverflow)?;

		supplier_data.index = supply_index;

		MntSupplierData::<T>::insert(supplier, supplier_data);

		<Pallet<T>>::deposit_event(Event::MntDistributedToSupplier(
			wrapped_id,
			supplier.clone(),
			supplier_delta,
			supply_index,
		));

		Ok(())
	}

	fn update_mnt_supply_index(underlying_id: CurrencyId) -> DispatchResult {
		// delta_blocks = current_block_number - supply_state.block_number
		// mnt_accrued = delta_block * mnt_speed
		// ratio = mnt_accrued / mtoken.total_supply()
		// supply_state.index += ratio
		// supply_state.block_number = current_block_number

		let current_block = frame_system::Module::<T>::block_number();
		let supply_speed = MntSpeeds::<T>::get(underlying_id).ok_or(Error::<T>::StorageIsCorrupted)?;
		let mut supply_state = MntPoolsState::<T>::get(underlying_id)
			.ok_or(Error::<T>::StorageIsCorrupted)?
			.supply_state;
		let delta_blocks = current_block
			.checked_sub(&supply_state.block_number)
			.ok_or(Error::<T>::NumOverflow)?;

		if delta_blocks != T::BlockNumber::zero() && supply_speed != FixedU128::zero() {
			// TODO rework this
			let wrapped_id = T::EnabledCurrencyPair::get()
				.iter()
				.find(|currency_pair| currency_pair.underlying_id == underlying_id)
				.ok_or(Error::<T>::NotValidUnderlyingAssetId)?
				.wrapped_id;

			let block_delta_as_usize = TryInto::<u32>::try_into(delta_blocks)
				.ok()
				.expect("blockchain will not exceed 2^32 blocks; qed");

			let block_delta = Rate::saturating_from_integer(block_delta_as_usize);

			let mnt_accrued = supply_speed.checked_mul(&block_delta).ok_or(Error::<T>::NumOverflow)?;

			let total_tokens_supply = Rate::checked_from_integer(T::MultiCurrency::total_issuance(wrapped_id))
				.ok_or(Error::<T>::NumOverflow)?;

			let ratio = mnt_accrued
				.checked_div(&total_tokens_supply)
				.ok_or(Error::<T>::NumOverflow)?;

			supply_state.index = supply_state.index.checked_add(&ratio).ok_or(Error::<T>::NumOverflow)?;
		}
		supply_state.block_number = current_block;

		MntPoolsState::<T>::try_mutate(underlying_id, |pool| -> DispatchResult {
			pool.as_mut().ok_or(Error::<T>::StorageIsCorrupted)?.supply_state = supply_state;
			Ok(())
		})
	}

	/// Calculate utilities for enabled pools and sum of all pools utilities
	///
	/// returns (Vector<CurrencyId, pool_utility>, sum_of_all_pools_utilities)
	fn calculate_enabled_pools_utilities() -> result::Result<(Vec<(CurrencyId, Balance)>, Balance), DispatchError> {
		let minted_pools = MntSpeeds::<T>::iter();
		let mut result: Vec<(CurrencyId, Balance)> = Vec::new();
		let mut total_utility: Balance = Balance::zero();
		for (currency_id, _) in minted_pools {
			let underlying_price =
				T::PriceSource::get_underlying_price(currency_id).ok_or(Error::<T>::GetUnderlyingPriceFail)?;
			let total_borrow = T::LiquidityPoolsManager::get_pool_total_borrowed(currency_id);

			// utility = m_tokens_total_borrows * asset_price
			let utility = Price::from_inner(total_borrow)
				.checked_mul(&underlying_price)
				.map(|x| x.into_inner())
				.ok_or(Error::<T>::NumOverflow)?;

			total_utility = total_utility.checked_add(utility).ok_or(Error::<T>::NumOverflow)?;

			result.push((currency_id, utility));
		}
		Ok((result, total_utility))
	}

	/// Recalculate MNT speeds
	fn refresh_mnt_speeds() -> DispatchResult {
		// TODO Add update indexes here when it will be implemented
		let (pool_utilities, sum_of_all_utilities) = Self::calculate_enabled_pools_utilities()?;
		let sum_of_all_utilities = Rate::from_inner(sum_of_all_utilities);
		if sum_of_all_utilities == Rate::zero() {
			// There is nothing to calculate.
			return Ok(());
		}
		let mnt_rate = Self::mnt_rate();
		for (currency_id, utility) in pool_utilities {
			let utility = Rate::from_inner(utility);
			let utility_fraction = utility
				.checked_div(&sum_of_all_utilities)
				.ok_or(Error::<T>::NumOverflow)?;
			let pool_mnt_speed = mnt_rate.checked_mul(&utility_fraction).ok_or(Error::<T>::NumOverflow)?;
			MntSpeeds::<T>::insert(currency_id, pool_mnt_speed);
		}
		Ok(())
	}
}
