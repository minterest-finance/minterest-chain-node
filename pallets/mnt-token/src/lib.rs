//! # MNT token Module
//!
//! Provides functionality for minting MNT tokens.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, sp_std::cmp::Ordering, transactional};
use frame_system::pallet_prelude::*;
use minterest_primitives::{currency::MNT, Balance, CurrencyId, Price, Rate};
pub use module::*;
use orml_traits::MultiCurrency;
use pallet_traits::{ControllerAPI, LiquidityPoolsManager, MntManager, PoolsManager, PriceProvider};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero},
	DispatchResult, FixedPointNumber, FixedU128,
};
use sp_std::{convert::TryInto, result, vec::Vec};
pub mod weights;
pub use weights::WeightInfo;

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
	/// Index that represents MNT tokens distribution for the whole pool.
	/// User MNT tokens distribution is based on this index.
	pub mnt_distribution_index: Rate,
	/// The block number the index was last updated at
	pub index_updated_at_block: T::BlockNumber,
}

impl<T: Config> MntState<T> {
	fn new() -> MntState<T> {
		MntState {
			mnt_distribution_index: Rate::one(), // initial index
			index_updated_at_block: frame_system::Module::<T>::block_number(),
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

type BalanceResult = result::Result<Balance, DispatchError>;

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

		/// The `MultiCurrency` implementation for wrapped.
		type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

		/// Public API of controller pallet
		type ControllerAPI: ControllerAPI<Self::AccountId>;

		#[pallet::constant]
		/// The Mnt-token's account id, keep assets that should be distributed to users
		type MntTokenAccountId: Get<Self::AccountId>;

		#[pallet::constant]
		// The MntSpeed update period.
		type SpeedRefreshPeriod: Get<Self::BlockNumber>;

		/// Weight information for the extrinsics.
		type MntTokenWeightInfo: WeightInfo;

		type PoolsManager: PoolsManager<Self::AccountId>;
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

	/// The threshold above which the flywheel transfers MNT
	#[pallet::storage]
	#[pallet::getter(fn mnt_claim_threshold)]
	pub(crate) type MntClaimThreshold<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// MNT minting speed for each pool
	/// Doubling this number shows how much MNT goes to all suppliers and borrowers of a particular
	/// pool.
	#[pallet::storage]
	#[pallet::getter(fn mnt_speeds)]
	pub(crate) type MntSpeeds<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Balance, ValueQuery>;

	/// Index + block_number need for generating and distributing new MNT tokens for pool
	#[pallet::storage]
	#[pallet::getter(fn mnt_pools_state)]
	pub(crate) type MntPoolsState<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, MntPoolState<T>, ValueQuery>;

	/// Use for accruing MNT tokens for supplier
	#[pallet::storage]
	#[pallet::getter(fn mnt_supplier_index)]
	pub(crate) type MntSupplierIndex<T: Config> =
		StorageDoubleMap<_, Twox64Concat, CurrencyId, Twox64Concat, T::AccountId, Rate, OptionQuery>;

	/// Use for accruing MNT tokens for borrower
	#[pallet::storage]
	#[pallet::getter(fn mnt_borrower_index)]
	pub(crate) type MntBorrowerIndex<T: Config> =
		StorageDoubleMap<_, Twox64Concat, CurrencyId, Twox64Concat, T::AccountId, Rate, ValueQuery>;

	/// Place where accrued MNT tokens are kept for each user
	#[pallet::storage]
	#[pallet::getter(fn mnt_accrued)]
	pub(crate) type MntAccrued<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub mnt_rate: Balance,
		pub mnt_claim_threshold: Balance,
		pub minted_pools: Vec<CurrencyId>,
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				mnt_rate: Balance::zero(),
				mnt_claim_threshold: Balance::zero(),
				minted_pools: vec![],
				_phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			MntRate::<T>::put(&self.mnt_rate);
			MntClaimThreshold::<T>::put(&self.mnt_claim_threshold);
			for currency_id in &self.minted_pools {
				MntSpeeds::<T>::insert(currency_id, Balance::zero());
				MntPoolsState::<T>::insert(currency_id, MntPoolState::new());
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_finalize(block: T::BlockNumber) {
			if block % T::SpeedRefreshPeriod::get() == T::BlockNumber::zero() {
				if let Err(msg) = Pallet::<T>::refresh_mnt_speeds() {
					debug::error!(
						"MntToken module: Cannot run refresh_mnt_speed() at {:?}: {:?}",
						block,
						msg
					)
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::MntTokenWeightInfo::enable_mnt_minting())]
		#[transactional]
		/// Enable MNT minting for pool and recalculate MntSpeeds
		pub fn enable_mnt_minting(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				currency_id.is_supported_underlying_asset(),
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

		#[pallet::weight(T::MntTokenWeightInfo::disable_mnt_minting())]
		#[transactional]
		/// Disable MNT minting for pool and recalculate MntSpeeds
		pub fn disable_mnt_minting(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(
				currency_id.is_supported_underlying_asset(),
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

		#[pallet::weight(T::MntTokenWeightInfo::set_mnt_rate())]
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
	/// Gets module account id.
	pub fn get_account_id() -> T::AccountId {
		T::MntTokenAccountId::get()
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
	pub fn refresh_mnt_speeds() -> DispatchResult {
		MntSpeeds::<T>::iter().try_for_each(|(pool_id, _)| -> DispatchResult {
			Self::update_mnt_supply_index(pool_id)?;
			Self::update_mnt_borrow_index(pool_id)?;
			Ok(())
		})?;

		let (pool_utilities, sum_of_all_utilities) = Self::calculate_enabled_pools_utilities()?;
		if sum_of_all_utilities.is_zero() {
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

	/// Transfer MNT tokens to user balance if they are above the threshold.
	/// Otherwise, put them into internal storage.
	///
	/// - `user`: MNT tokens recipient.
	/// - `user_accrued`: The total amount of accrued tokens.
	/// - `distribute_all`: boolean, distribute all or part of accrued MNT tokens.
	fn transfer_mnt(user: &T::AccountId, user_accrued: Balance, distribute_all: bool) -> DispatchResult {
		//TODO: Need to discuss what we should do.
		// Erorr/Event/save money to MntAccrued/stop producing mnt tokens

		let threshold = match distribute_all {
			true => Balance::zero(),
			false => MntClaimThreshold::<T>::get(),
		};

		if user_accrued >= threshold && user_accrued > 0 {
			let mnt_treasury_balance = T::MultiCurrency::free_balance(MNT, &Self::get_account_id());
			if user_accrued <= mnt_treasury_balance {
				T::MultiCurrency::transfer(MNT, &Self::get_account_id(), &user, user_accrued)?;
				MntAccrued::<T>::remove(user); // set to 0
			}
		} else {
			MntAccrued::<T>::insert(user, user_accrued);
		}
		Ok(())
	}
}

// RPC methods
impl<T: Config> Pallet<T> {
	/// Gets MNT accrued but not yet transferred to user.
	pub fn get_unclaimed_mnt_balance(account_id: &T::AccountId) -> BalanceResult {
		let accrued_mnt =
			MntSpeeds::<T>::iter().try_fold(Balance::zero(), |current_accrued, (pool_id, _)| -> BalanceResult {
				Self::update_mnt_borrow_index(pool_id)?;
				let accrued_borrow_mnt = Self::distribute_borrower_mnt(pool_id, account_id, true)?;
				Self::update_mnt_supply_index(pool_id)?;
				let accrued_supply_mnt = Self::distribute_supplier_mnt(pool_id, account_id, true)?;
				Ok(current_accrued + accrued_borrow_mnt + accrued_supply_mnt)
			})?;
		Ok(accrued_mnt)
	}
}

impl<T: Config> MntManager<T::AccountId> for Pallet<T> {
	/// Update mnt supply index for pool
	fn update_mnt_supply_index(underlying_id: CurrencyId) -> DispatchResult {
		// block_delta = current_block_number - supply_state.index_updated_at_block
		// mnt_accrued = block_delta * mnt_speed
		// ratio = mnt_accrued / mtoken.total_supply()
		// supply_state.mnt_distribution_index += ratio
		// supply_state.index_updated_at_block = current_block_number

		let current_block = frame_system::Module::<T>::block_number();
		let mut pool_state = MntPoolsState::<T>::get(underlying_id);
		let block_delta = current_block
			.checked_sub(&pool_state.supply_state.index_updated_at_block)
			.ok_or(Error::<T>::NumOverflow)?;

		if block_delta.is_zero() {
			// Index for current block was already calculated
			return Ok(());
		}

		let mnt_speed = MntSpeeds::<T>::get(underlying_id);
		if !mnt_speed.is_zero() {
			let wrapped_asset_id = underlying_id
				.wrapped_asset()
				.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;

			let block_delta_as_u128 = TryInto::<u128>::try_into(block_delta).or(Err(Error::<T>::InternalError))?;

			let mnt_accrued = mnt_speed
				.checked_mul(block_delta_as_u128)
				.ok_or(Error::<T>::NumOverflow)?;

			let total_tokens_supply = T::MultiCurrency::total_issuance(wrapped_asset_id);

			let ratio = match total_tokens_supply.cmp(&Balance::zero()) {
				Ordering::Greater => {
					Rate::checked_from_rational(mnt_accrued, total_tokens_supply).ok_or(Error::<T>::NumOverflow)?
				}
				_ => Rate::zero(),
			};

			pool_state.supply_state.mnt_distribution_index = pool_state
				.supply_state
				.mnt_distribution_index
				.checked_add(&ratio)
				.ok_or(Error::<T>::NumOverflow)?;
		}
		pool_state.supply_state.index_updated_at_block = current_block;

		MntPoolsState::<T>::insert(underlying_id, pool_state);
		Ok(())
	}

	/// Update mnt borrow index for pool
	fn update_mnt_borrow_index(underlying_id: CurrencyId) -> DispatchResult {
		// block_delta = current_block_number - borrow_state.index_updated_at_block
		// mnt_accrued = delta_blocks * mnt_speed
		// borrow_amount - mtoken.total_borrows() / liquidity_pool_borrow_index
		// ratio = mnt_accrued / borrow_amount
		// borrow_state.mnt_distribution_index(for current pool) += ratio
		// borrow_state.index_updated_at_block = current_block_number

		let current_block = frame_system::Module::<T>::block_number();
		let mut pool_state = MntPoolsState::<T>::get(underlying_id);
		let block_delta = current_block
			.checked_sub(&pool_state.borrow_state.index_updated_at_block)
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

			let ratio = match borrow_amount.cmp(&Rate::zero()) {
				Ordering::Greater => Rate::from_inner(mnt_accrued)
					.checked_div(&borrow_amount)
					.ok_or(Error::<T>::NumOverflow)?,
				_ => Rate::zero(),
			};

			pool_state.borrow_state.mnt_distribution_index = pool_state
				.borrow_state
				.mnt_distribution_index
				.checked_add(&ratio)
				.ok_or(Error::<T>::NumOverflow)?;
		}

		pool_state.borrow_state.index_updated_at_block = current_block;
		MntPoolsState::<T>::insert(underlying_id, pool_state);
		Ok(())
	}

	/// Distribute mnt token to supplier. It should be called after update_mnt_supply_index
	fn distribute_supplier_mnt(
		underlying_id: CurrencyId,
		supplier: &T::AccountId,
		distribute_all: bool,
	) -> BalanceResult {
		// delta_index = mnt_distribution_index - mnt_supplier_index
		// supplier_delta = supplier_mtoken_balance * delta_index
		// supplier_mnt_balance += supplier_delta
		// mnt_supplier_index = mnt_distribution_index
		let supply_index = MntPoolsState::<T>::get(underlying_id)
			.supply_state
			.mnt_distribution_index;

		let supplier_index = MntSupplierIndex::<T>::get(underlying_id, supplier).unwrap_or_else(Rate::one);

		let delta_index = supply_index
			.checked_sub(&supplier_index)
			.ok_or(Error::<T>::NumOverflow)?;

		let wrapped_asset_id = underlying_id
			.wrapped_asset()
			.ok_or(Error::<T>::NotValidUnderlyingAssetId)?;

		// We use total_balance (not free balance). Because sum of balances should be equal to
		// total_issuance. Otherwise, mnt_rate calculation will not be correct.
		// (see total_tokens_supply in update_mnt_supply_index)
		let supplier_balance = Rate::from_inner(T::MultiCurrency::total_balance(wrapped_asset_id, supplier));

		let supplier_delta = delta_index
			.checked_mul(&supplier_balance)
			.ok_or(Error::<T>::NumOverflow)?;

		let mut supplier_mnt_accrued = MntAccrued::<T>::get(supplier);

		supplier_mnt_accrued = supplier_mnt_accrued
			.checked_add(supplier_delta.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		MntSupplierIndex::<T>::insert(underlying_id, supplier, supply_index);
		Self::transfer_mnt(supplier, supplier_mnt_accrued, distribute_all)?;

		Self::deposit_event(Event::MntDistributedToSupplier(
			underlying_id,
			supplier.clone(),
			supplier_delta.into_inner(),
			supply_index,
		));

		Ok(supplier_mnt_accrued)
	}

	/// Distribute MNT token to borrower. It should be called after update_mnt_borrow_index.
	/// Borrowers will not begin to accrue tokens till the first interaction with the protocol.
	///
	/// - `underlying_id`: The pool in which the borrower is acting;
	/// - `borrower`: The AccountId of the borrower to distribute MNT to.
	fn distribute_borrower_mnt(
		underlying_id: CurrencyId,
		borrower: &T::AccountId,
		distribute_all: bool,
	) -> BalanceResult {
		// borrower_amount = account_borrow_balance / liquidity_pool_borrow_index
		// delta_index = mnt_distribution_index(for current pool) - borrower_index
		// borrower_delta = borrower_amount * delta_index
		// borrower_accrued += borrower_delta
		// borrower_index = mnt_distribution_index(for current pool)

		let borrower_index = MntBorrowerIndex::<T>::get(underlying_id, borrower);
		let pool_borrow_state = MntPoolsState::<T>::get(underlying_id).borrow_state;
		// Update borrower index
		MntBorrowerIndex::<T>::insert(underlying_id, borrower, pool_borrow_state.mnt_distribution_index);
		if borrower_index.is_zero() {
			// This is first interaction with protocol
			return Ok(Balance::zero());
		}

		let borrow_balance = T::ControllerAPI::borrow_balance_stored(&borrower, underlying_id)?;
		let pool_borrow_index = T::LiquidityPoolsManager::get_pool_borrow_index(underlying_id);
		let borrower_amount = Price::from_inner(borrow_balance)
			.checked_div(&pool_borrow_index)
			.ok_or(Error::<T>::NumOverflow)?;

		let delta_index = pool_borrow_state
			.mnt_distribution_index
			.checked_sub(&borrower_index)
			.ok_or(Error::<T>::NumOverflow)?;

		if delta_index == Rate::zero() {
			return Ok(Balance::zero());
		}

		let borrower_delta = borrower_amount
			.checked_mul(&delta_index)
			.ok_or(Error::<T>::NumOverflow)?;

		let mut borrower_mnt_accrued = MntAccrued::<T>::get(borrower);
		borrower_mnt_accrued = borrower_mnt_accrued
			.checked_add(borrower_delta.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		Self::transfer_mnt(borrower, borrower_mnt_accrued, distribute_all)?;

		Self::deposit_event(Event::MntDistributedToBorrower(
			underlying_id,
			borrower.clone(),
			borrower_delta.into_inner(),
			pool_borrow_state.mnt_distribution_index,
		));
		Ok(borrower_mnt_accrued)
	}

	fn get_mnt_borrow_supply_apy(pool_id: CurrencyId) -> Result<(Price, Price), DispatchError> {
		let total_borrow = T::LiquidityPoolsManager::get_pool_total_borrowed(pool_id);

		if total_borrow == 0 {
			return Ok((Price::zero(), Price::zero()));
		}

		Self::refresh_mnt_speeds()?;
		let mnt_speed = MntSpeeds::<T>::get(pool_id);
		let mnt_price = T::PriceSource::get_underlying_price(MNT).ok_or(Error::<T>::GetUnderlyingPriceFail)?;

		let oracle_price = T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::GetUnderlyingPriceFail)?;



		let total_cash = T::PoolsManager::get_pool_available_liquidity(pool_id);
		let total_protocol_interest = T::LiquidityPoolsManager::get_pool_total_protocol_interest(pool_id);
		let total_supply = FixedU128::from_inner(total_cash - total_protocol_interest + total_borrow);

		let borrow_apy = FixedU128::from_inner(mnt_speed) * mnt_price 
						/ (FixedU128::from_inner(total_borrow) * oracle_price);

		let supply_apy = FixedU128::from_inner(mnt_speed) * mnt_price / (total_supply * oracle_price);

		Ok((borrow_apy, supply_apy))
	}
}
