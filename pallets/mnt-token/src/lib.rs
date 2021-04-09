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
use pallet_traits::{ControllerAPI, LiquidityPoolsManager, PriceProvider};
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

/// Representation of supply/borrow pool state
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MntState<T: Config> {
	/// Index that represents MNT tokens that distributes for whole pool.
	/// There is calculation MNT tokens for each user based on this index.
	pub index: FixedU128,
	/// The block number the index was last updated at
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

/// Each pool state contains supply and borrow part
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
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

impl<T: Config> Default for MntPoolState<T> {
	fn default() -> Self {
		Self::new()
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
		type EnabledUnderlyingAssetsIds: Get<Vec<CurrencyId>>;

		/// Enabled currency pairs.
		type EnabledCurrencyPair: Get<Vec<CurrencyPair>>;

		/// The `MultiCurrency` implementation for wrapped.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// Public API of controller pallet
		type ControllerAPI: ControllerAPI<Self::AccountId>;
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
		InternalError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Change rate event (old rate, new rate)
		NewMntRate(Balance, Balance),

		/// MNT minting enabled for pool
		MntMintingEnabled(CurrencyId),

		/// MNT minting disabled for pool
		MntMintingDisabled(CurrencyId),

		/// Emitted when MNT is distributed to a supplier
		/// (pool id, receiver, amount of distributed tokens, supply index)
		MntDistributedToSupplier(CurrencyId, T::AccountId, Balance, Rate),

		/// Emitted when MNT is distributed to a borrower
		/// (pool id, receiver, amount of distributed tokens, index)
		MntDistributedToBorrower(CurrencyId, T::AccountId, Balance, Rate),
	}

	/// The rate at which the flywheel distributes MNT, per block.
	/// Doubling this number shows how much MNT goes to all suppliers and borrowers from all pools.
	#[pallet::storage]
	#[pallet::getter(fn mnt_rate)]
	type MntRate<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// MNT minting speed for each pool
	/// Doubling this number shows how much MNT goes to all suppliers and borrowers of particular
	/// pool.
	#[pallet::storage]
	#[pallet::getter(fn mnt_speeds)]
	pub(crate) type MntSpeeds<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	// TODO Could I merge MntSpeeds and MntPoolsState storage into one?
	/// Index + block_number need for generating and distributing new MNT tokens for pool
	#[pallet::storage]
	#[pallet::getter(fn mnt_pools_state)]
	pub(crate) type MntPoolsState<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, MntPoolState<T>, ValueQuery>;

	/// Use for accruing MNT tokens for supplier
	#[pallet::storage]
	#[pallet::getter(fn mnt_supplier_index)]
	pub(crate) type MntSupplierIndex<T: Config> =
		StorageDoubleMap<_, Twox64Concat, CurrencyId, Twox64Concat, T::AccountId, Rate, OptionQuery>;

	/// Place where accrued MNT token are keeping for each user
	#[pallet::storage]
	#[pallet::getter(fn mnt_accrued)]
	pub(crate) type MntAccrued<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	/// Use for accruing MNT tokens for borrower
	#[pallet::storage]
	#[pallet::getter(fn mnt_borrower_index)]
	pub(crate) type MntBorrowerIndex<T: Config> =
		StorageDoubleMap<_, Twox64Concat, CurrencyId, Twox64Concat, T::AccountId, Rate, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub mnt_rate: Balance,
		pub minted_pools: Vec<CurrencyId>,
		pub phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				mnt_rate: Balance::zero(),
				minted_pools: vec![],
				phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			MntRate::<T>::put(&self.mnt_rate);
			for currency_id in &self.minted_pools {
				MntSpeeds::<T>::insert(currency_id, Balance::zero());
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
				T::EnabledUnderlyingAssetsIds::get()
					.into_iter()
					.any(|asset_id| asset_id == currency_id),
				Error::<T>::NotValidUnderlyingAssetId
			);
			ensure!(
				!MntSpeeds::<T>::contains_key(currency_id),
				Error::<T>::MntMintingAlreadyEnabled
			);
			MntSpeeds::<T>::insert(currency_id, Balance::zero());
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
				T::EnabledUnderlyingAssetsIds::get()
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
		pub fn set_mnt_rate(origin: OriginFor<T>, rate: Balance) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			let old_rate = MntRate::<T>::get();
			MntRate::<T>::put(rate);
			Self::refresh_mnt_speeds()?;
			Self::deposit_event(Event::NewMntRate(old_rate, rate));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Distribute mnt token to borrower. It should be called after update_mnt_borrow_index
	#[allow(dead_code)] // TODO remove this
	fn distribute_borrower_mnt(underlying_id: CurrencyId, borrower: &T::AccountId) -> DispatchResult {
		// borrower_amount = account_borrow_balance / pool_borrow_index
		// delta_index = pool_borrow_index - borrower_index
		// borrower_delta = borrower_amount * delta_index
		// borrower_accrued += borrower_delta
		// borrower_index = borrow_index

		let borrow_balance = T::ControllerAPI::borrow_balance_stored(&borrower, underlying_id)?;
		let pool_borrow_index = T::LiquidityPoolsManager::get_pool_borrow_index(underlying_id);
		let borrower_amount = Price::from_inner(borrow_balance)
			.checked_div(&pool_borrow_index)
			.ok_or(Error::<T>::NumOverflow)?;

		let mut borrower_index = MntBorrowerIndex::<T>::get(underlying_id, borrower).unwrap_or_else(|| Rate::one());

		let pool_borrow_state = MntPoolsState::<T>::get(underlying_id).borrow_state;
		let delta_index = pool_borrow_state
			.index
			.checked_sub(&borrower_index)
			.ok_or(Error::<T>::NumOverflow)?;

		if delta_index == Rate::zero() {
			return Ok(());
		}

		let borrower_delta = borrower_amount
			.checked_mul(&delta_index)
			.ok_or(Error::<T>::NumOverflow)?;

		let mut borrower_mnt_accrued = MntAccrued::<T>::get(borrower);
		borrower_mnt_accrued = borrower_mnt_accrued
			.checked_add(borrower_delta.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		borrower_index = pool_borrow_state.index;

		MntBorrowerIndex::<T>::insert(underlying_id, borrower, borrower_index);
		MntAccrued::<T>::insert(borrower, borrower_mnt_accrued);

		Self::deposit_event(Event::MntDistributedToBorrower(
			underlying_id,
			borrower.clone(),
			borrower_delta.into_inner(),
			borrower_index,
		));
		Ok(())
	}

	/// Update mnt borrow index for pool
	#[allow(dead_code)] // TODO remove this
	fn update_mnt_borrow_index(underlying_id: CurrencyId) -> DispatchResult {
		// block_delta = current_block_number - bottow_state.block_number
		// mnt_accrued = delta_blocks * mnt_speed
		// borrow_amount - mtoken.total_borrows() / market_borrow_index
		// ratio = mnt_accrued / borrow_amount
		// borrow_state.index += ratio
		// borrow_state.block_number = current_block_number

		let current_block = frame_system::Module::<T>::block_number();
		let mut borrow_state = MntPoolsState::<T>::get(underlying_id).borrow_state;
		let block_delta = current_block
			.checked_sub(&borrow_state.block_number)
			.ok_or(Error::<T>::NumOverflow)?;

		if block_delta.is_zero() {
			// Index for current block was already calculated
			return Ok(());
		}

		let mnt_speed = MntSpeeds::<T>::get(underlying_id);
		if !mnt_speed.is_zero() {
			let block_delta_as_u128 = TryInto::<u128>::try_into(block_delta).or(Err(Error::<T>::InternalError))?;

			let mnt_accrued = mnt_speed
				.checked_mul(block_delta_as_u128)
				.ok_or(Error::<T>::NumOverflow)?;

			let total_borrowed_as_rate =
				Rate::from_inner(T::LiquidityPoolsManager::get_pool_total_borrowed(underlying_id));

			let borrow_amount = total_borrowed_as_rate
				.checked_div(&T::LiquidityPoolsManager::get_pool_borrow_index(underlying_id))
				.ok_or(Error::<T>::NumOverflow)?;

			let ratio = Rate::from_inner(mnt_accrued)
				.checked_div(&borrow_amount)
				.ok_or(Error::<T>::NumOverflow)?;

			borrow_state.index = borrow_state.index.checked_add(&ratio).ok_or(Error::<T>::NumOverflow)?;
		}
		borrow_state.block_number = current_block;
		MntPoolsState::<T>::try_mutate(underlying_id, |pool| -> DispatchResult {
			pool.borrow_state = borrow_state;
			Ok(())
		})
	}

	/// Distribute mnt token to supplier. It should be called after update_mnt_supply_index
	#[allow(dead_code)] // TODO remove this
	fn distribute_supplier_mnt(underlying_id: CurrencyId, supplier: &T::AccountId) -> DispatchResult {
		// delta_index = mnt_supply_index - mnt_supplier_index
		// supplier_delta = supplier_mtoken_balance * delta_index
		// supplier_mnt_balance += supplier_delta
		// mnt_supplier_index = mnt_supply_index
		let supply_index = MntPoolsState::<T>::get(underlying_id).supply_state.index;

		let mut supplier_index = MntSupplierIndex::<T>::get(underlying_id, supplier).unwrap_or_else(|| Rate::one());

		let delta_index = supply_index
			.checked_sub(&supplier_index)
			.ok_or(Error::<T>::NumOverflow)?;

		// This should be reworked. TODO MIN-185
		let wrapped_id = T::EnabledCurrencyPair::get()
			.iter()
			.find(|currency_pair| currency_pair.underlying_id == underlying_id)
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?
			.wrapped_id;

		// We use total_balance (not free balance). Because sum of balances should be equal to
		// total_issuance. Otherwise, mnt_rate calculating will not correct.
		// (see total_tokens_supply in update_mnt_supply_index)
		let supplier_balance = Rate::from_inner(T::MultiCurrency::total_balance(wrapped_id, supplier));

		let supplier_delta = delta_index
			.checked_mul(&supplier_balance)
			.ok_or(Error::<T>::NumOverflow)?;

		let mut supplier_mnt_accrued = MntAccrued::<T>::get(supplier);

		supplier_mnt_accrued = supplier_mnt_accrued
			.checked_add(supplier_delta.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		supplier_index = supply_index;

		MntAccrued::<T>::insert(supplier, supplier_mnt_accrued);
		MntSupplierIndex::<T>::insert(underlying_id, supplier, supplier_index);

		Self::deposit_event(Event::MntDistributedToSupplier(
			underlying_id,
			supplier.clone(),
			supplier_delta.into_inner(),
			supply_index,
		));

		Ok(())
	}

	/// Update mnt supply index for pool
	#[allow(dead_code)] // TODO remove this
	fn update_mnt_supply_index(underlying_id: CurrencyId) -> DispatchResult {
		// block_delta = current_block_number - supply_state.block_number
		// mnt_accrued = block_delta * mnt_speed
		// ratio = mnt_accrued / mtoken.total_supply()
		// supply_state.index += ratio
		// supply_state.block_number = current_block_number

		let current_block = frame_system::Module::<T>::block_number();
		let mut supply_state = MntPoolsState::<T>::get(underlying_id).supply_state;
		let block_delta = current_block
			.checked_sub(&supply_state.block_number)
			.ok_or(Error::<T>::NumOverflow)?;

		if block_delta.is_zero() {
			// Index for current block was already calculated
			return Ok(());
		}

		let mnt_speed = MntSpeeds::<T>::get(underlying_id);
		if !mnt_speed.is_zero() {
			// This should be reworked. TODO MIN-185
			let wrapped_id = T::EnabledCurrencyPair::get()
				.iter()
				.find(|currency_pair| currency_pair.underlying_id == underlying_id)
				.ok_or(Error::<T>::NotValidUnderlyingAssetId)?
				.wrapped_id;

			let block_delta_as_u128 = TryInto::<u128>::try_into(block_delta).or(Err(Error::<T>::InternalError))?;

			let mnt_accrued = mnt_speed
				.checked_mul(block_delta_as_u128)
				.ok_or(Error::<T>::NumOverflow)?;

			let total_tokens_supply = T::MultiCurrency::total_issuance(wrapped_id);

			let ratio = Rate::checked_from_rational(mnt_accrued, total_tokens_supply).ok_or(Error::<T>::NumOverflow)?;

			supply_state.index = supply_state.index.checked_add(&ratio).ok_or(Error::<T>::NumOverflow)?;
		}
		supply_state.block_number = current_block;

		MntPoolsState::<T>::try_mutate(underlying_id, |pool| -> DispatchResult {
			pool.supply_state = supply_state;
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
		if sum_of_all_utilities == Balance::zero() {
			// There is nothing to calculate.
			return Ok(());
		}
		let mnt_rate = Self::mnt_rate();
		for (currency_id, utility) in pool_utilities {
			let utility_fraction = Rate::saturating_from_rational(utility, sum_of_all_utilities);
			let pool_mnt_speed = Rate::from_inner(mnt_rate)
				.checked_mul(&utility_fraction)
				.ok_or(Error::<T>::NumOverflow)?;
			MntSpeeds::<T>::insert(currency_id, pool_mnt_speed.into_inner());
		}
		Ok(())
	}
}
