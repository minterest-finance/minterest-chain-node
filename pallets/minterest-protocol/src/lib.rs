//! # Minterest Protocol Module
//!
//! ## Overview
//!
//! TODO: add overview.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::traits::Contains;
use frame_support::{pallet_prelude::*, transactional};
use frame_system::{ensure_signed, pallet_prelude::*};
use minterest_primitives::{Balance, CurrencyId, Operation};
use orml_traits::MultiCurrency;
use pallet_traits::{Borrowing, PoolsManager};
use sp_runtime::{
	traits::{BadOrigin, Zero},
	DispatchError, DispatchResult,
};
use sp_std::cmp::Ordering;
use sp_std::result;

pub use module::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Controller<T> = controller::Module<T>;
type TokensResult = result::Result<(Balance, CurrencyId, Balance), DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + liquidity_pools::Config + controller::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Basic borrowing functions
		type Borrowing: Borrowing<Self::AccountId>;

		/// The basic liquidity pools.
		type ManagerLiquidityPools: PoolsManager<Self::AccountId>;

		/// The origin which may call deposit/redeem/borrow/repay in Whitelist mode.
		type WhitelistMembers: Contains<Self::AccountId>;

		type ProtocolWeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,
		/// The currency is not enabled in wrapped protocol.
		NotValidWrappedTokenId,
		/// There is not enough liquidity available in the pool.
		NotEnoughLiquidityAvailable,
		/// Insufficient wrapped tokens in the user account.
		NotEnoughWrappedTokens,
		/// User did not make deposit (no mTokens).
		NumberOfWrappedTokensIsZero,
		/// Insufficient underlying assets in the user account.
		NotEnoughUnderlyingsAssets,
		/// Number overflow in calculation.
		NumOverflow,
		/// An internal failure occurred in the execution of the Accrue Interest function.
		AccrueInterestFailed,
		/// Deposit was blocked due to Controller rejection.
		DepositControllerRejection,
		/// Redeem was blocked due to Controller rejection.
		RedeemControllerRejection,
		/// Transaction with zero balance is not allowed.
		ZeroBalanceTransaction,
		/// User is trying repay more than he borrowed.
		RepayAmountToBig,
		/// This pool is already collateral.
		AlreadyCollateral,
		/// This pool has already been disabled as a collateral.
		AlreadyDisabledCollateral,
		/// The user has an outstanding borrow. Cannot be disabled as collateral.
		CanotBeDisabledAsCollateral,
		/// The user has not deposited funds into the pool.
		CanotBeEnabledAsCollateral,
		/// Operation (deposit, redeem, borrow, repay) is paused.
		OperationPaused,
		/// The user is trying to transfer tokens to self
		CannotTransferToSelf,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Underlying assets added to pool and wrapped tokens minted: \[who, underlying_asset_id,
		/// underlying_amount, wrapped_currency_id, wrapped_amount\]
		Deposited(T::AccountId, CurrencyId, Balance, CurrencyId, Balance),

		/// Underlying assets and wrapped tokens redeemed: \[who, underlying_asset_id,
		/// underlying_amount, wrapped_currency_id, wrapped_amount\]
		Redeemed(T::AccountId, CurrencyId, Balance, CurrencyId, Balance),

		/// Borrowed a specific amount of the pool currency: \[who, underlying_asset_id,
		/// the_amount_to_be_borrowed\]
		Borrowed(T::AccountId, CurrencyId, Balance),

		/// Repaid a borrow on the specific pool, for the specified amount: \[who,
		/// underlying_asset_id, the_amount_repaid\]
		Repaid(T::AccountId, CurrencyId, Balance),

		/// Transferred specified amount on a specified pool from one account to another:
		/// \[who, receiver, wrapped_currency_id, wrapped_amount\]
		Transferred(T::AccountId, T::AccountId, CurrencyId, Balance),

		/// The user allowed the assets in the pool to be used as collateral: \[who, pool_id\]
		PoolEnabledAsCollateral(T::AccountId, CurrencyId),

		/// The user denies use the assets in pool as collateral: \[who, pool_id\]
		PoolDisabledCollateral(T::AccountId, CurrencyId),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfers an asset into the protocol. The user receives a quantity of mTokens equal
		/// to the underlying tokens supplied, divided by the current Exchange Rate.
		///
		/// - `underlying_asset_id`: CurrencyId of underlying assets to be transferred into the
		///   protocol.
		/// - `underlying_amount`: The amount of the asset to be supplied, in units of the
		///   underlying asset.
		#[pallet::weight(T::ProtocolWeightInfo::deposit_underlying())]
		#[transactional]
		pub fn deposit_underlying(
			origin: OriginFor<T>,
			underlying_asset_id: CurrencyId,
			#[pallet::compact] underlying_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			let (_, wrapped_id, wrapped_amount) = Self::do_deposit(&who, underlying_asset_id, underlying_amount)?;
			Self::deposit_event(Event::Deposited(
				who,
				underlying_asset_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Converts ALL mTokens into a specified quantity of the underlying asset, and returns them
		/// to the user. The amount of underlying tokens received is equal to the quantity of
		/// mTokens redeemed, multiplied by the current Exchange Rate.
		///
		/// - `underlying_asset_id`: CurrencyId of underlying assets to be redeemed.
		#[pallet::weight(T::ProtocolWeightInfo::redeem())]
		#[transactional]
		pub fn redeem(origin: OriginFor<T>, underlying_asset_id: CurrencyId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}
			let (underlying_amount, wrapped_id, wrapped_amount) =
				Self::do_redeem(&who, underlying_asset_id, Balance::zero(), Balance::zero(), true)?;
			Self::deposit_event(Event::Redeemed(
				who,
				underlying_asset_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Converts mTokens into a specified quantity of the underlying asset, and returns them to
		/// the user. The amount of mTokens redeemed is equal to the quantity of underlying tokens
		/// received, divided by the current Exchange Rate.
		///
		/// - `underlying_asset_id`: CurrencyId of underlying assets to be redeemed.
		/// - `underlying_amount`: The number of underlying assets to be redeemed.
		#[pallet::weight(T::ProtocolWeightInfo::redeem_underlying())]
		#[transactional]
		pub fn redeem_underlying(
			origin: OriginFor<T>,
			underlying_asset_id: CurrencyId,
			#[pallet::compact] underlying_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}
			let (_, wrapped_id, wrapped_amount) =
				Self::do_redeem(&who, underlying_asset_id, underlying_amount, Balance::zero(), false)?;
			Self::deposit_event(Event::Redeemed(
				who,
				underlying_asset_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Converts a specified quantity of mTokens into the underlying asset, and returns them to
		/// the user. The amount of underlying tokens received is equal to the quantity of mTokens
		/// redeemed, multiplied by the current Exchange Rate.
		///
		/// - `wrapped_id`: CurrencyId of mTokens to be redeemed.
		/// - `wrapped_amount`: The number of mTokens to be redeemed.
		#[pallet::weight(T::ProtocolWeightInfo::redeem_wrapped())]
		#[transactional]
		pub fn redeem_wrapped(
			origin: OriginFor<T>,
			wrapped_id: CurrencyId,
			#[pallet::compact] wrapped_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			let underlying_asset_id = <LiquidityPools<T>>::get_underlying_asset_id_by_wrapped_id(&wrapped_id)
				.map_err(|_| Error::<T>::NotValidWrappedTokenId)?;
			let (underlying_amount, wrapped_id, _) =
				Self::do_redeem(&who, underlying_asset_id, Balance::zero(), wrapped_amount, false)?;
			Self::deposit_event(Event::Redeemed(
				who,
				underlying_asset_id,
				underlying_amount,
				wrapped_id,
				wrapped_amount,
			));
			Ok(().into())
		}

		/// Borrowing a specific amount of the pool currency, provided that the borrower already
		/// deposited enough collateral.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be borrowed.
		/// - `underlying_amount`: The amount of the underlying asset to be borrowed.
		#[pallet::weight(T::ProtocolWeightInfo::borrow())]
		#[transactional]
		pub fn borrow(
			origin: OriginFor<T>,
			underlying_asset_id: CurrencyId,
			borrow_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			Self::do_borrow(&who, underlying_asset_id, borrow_amount)?;
			Self::deposit_event(Event::Borrowed(who, underlying_asset_id, borrow_amount));
			Ok(().into())
		}

		/// Repays a borrow on the specific pool, for the specified amount.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be repaid.
		/// - `repay_amount`: The amount of the underlying asset to be repaid.
		#[pallet::weight(T::ProtocolWeightInfo::repay())]
		#[transactional]
		pub fn repay(
			origin: OriginFor<T>,
			underlying_asset_id: CurrencyId,
			#[pallet::compact] repay_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			Self::do_repay(&who, &who, underlying_asset_id, repay_amount, false)?;
			Self::deposit_event(Event::Repaid(who, underlying_asset_id, repay_amount));
			Ok(().into())
		}

		/// Repays a borrow on the specific pool, for the all amount.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be repaid.
		#[pallet::weight(T::ProtocolWeightInfo::repay_all())]
		#[transactional]
		pub fn repay_all(origin: OriginFor<T>, underlying_asset_id: CurrencyId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			let repay_amount = Self::do_repay(&who, &who, underlying_asset_id, Balance::zero(), true)?;
			Self::deposit_event(Event::Repaid(who, underlying_asset_id, repay_amount));
			Ok(().into())
		}

		/// Transfers an asset into the protocol, reducing the target user's borrow balance.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be repaid.
		/// - `borrower`: The account which borrowed the asset to be repaid.
		/// - `repay_amount`: The amount of the underlying borrowed asset to be repaid.
		#[pallet::weight(T::ProtocolWeightInfo::repay_on_behalf())]
		#[transactional]
		pub fn repay_on_behalf(
			origin: OriginFor<T>,
			underlying_asset_id: CurrencyId,
			borrower: T::AccountId,
			repay_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			let repay_amount = Self::do_repay(&who, &borrower, underlying_asset_id, repay_amount, false)?;
			Self::deposit_event(Event::Repaid(who, underlying_asset_id, repay_amount));
			Ok(().into())
		}

		/// Transfers an asset within the pool.
		///
		/// - `receiver`: the account that will receive tokens.
		/// - `wrapped_id`: the currency ID of the wrapped asset to transfer.
		/// - `transfer_amount`: the amount of the wrapped asset to transfer.
		#[pallet::weight(T::ProtocolWeightInfo::transfer_wrapped())]
		#[transactional]
		pub fn transfer_wrapped(
			origin: OriginFor<T>,
			receiver: T::AccountId,
			wrapped_id: CurrencyId,
			transfer_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&who), BadOrigin);
			}

			Self::do_transfer(&who, &receiver, wrapped_id, transfer_amount)?;
			Self::deposit_event(Event::Transferred(who, receiver, wrapped_id, transfer_amount));
			Ok(().into())
		}

		/// Sender allowed the assets in the pool to be used as collateral.
		#[pallet::weight(T::ProtocolWeightInfo::enable_collateral())]
		#[transactional]
		pub fn enable_as_collateral(origin: OriginFor<T>, pool_id: CurrencyId) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&sender), BadOrigin);
			}

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			ensure!(
				!<LiquidityPools<T>>::check_user_available_collateral(&sender, pool_id),
				Error::<T>::AlreadyCollateral
			);

			// If user does not have assets in the pool, then he cannot enable as collateral the pool.
			let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&pool_id)?;
			let user_wrapped_balance = T::MultiCurrency::free_balance(wrapped_id, &sender);
			ensure!(user_wrapped_balance > 0, Error::<T>::CanotBeEnabledAsCollateral);

			<LiquidityPools<T>>::enable_as_collateral_internal(&sender, pool_id)?;
			Self::deposit_event(Event::PoolEnabledAsCollateral(sender, pool_id));
			Ok(().into())
		}

		/// Sender has denies use the assets in pool as collateral.
		#[pallet::weight(T::ProtocolWeightInfo::disable_collateral())]
		#[transactional]
		pub fn disable_collateral(origin: OriginFor<T>, pool_id: CurrencyId) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;

			if controller::WhitelistMode::<T>::get() {
				ensure!(T::WhitelistMembers::contains(&sender), BadOrigin);
			}

			ensure!(
				<LiquidityPools<T>>::is_enabled_underlying_asset_id(pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			ensure!(
				<LiquidityPools<T>>::check_user_available_collateral(&sender, pool_id),
				Error::<T>::AlreadyDisabledCollateral
			);

			let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&pool_id)?;
			let user_balance_wrapped_tokens = T::MultiCurrency::free_balance(wrapped_id, &sender);
			let user_balance_disabled_asset =
				<LiquidityPools<T>>::convert_from_wrapped(wrapped_id, user_balance_wrapped_tokens)?;

			// Check if the user will have enough collateral if he removes one of the collaterals.
			let (_, shortfall) =
				<Controller<T>>::get_hypothetical_account_liquidity(&sender, pool_id, user_balance_disabled_asset, 0)?;
			ensure!(!(shortfall > 0), Error::<T>::CanotBeDisabledAsCollateral);

			<LiquidityPools<T>>::disable_collateral_internal(&sender, pool_id)?;
			Self::deposit_event(Event::PoolDisabledCollateral(sender, pool_id));
			Ok(().into())
		}
	}
}

// Dispatchable calls implementation
impl<T: Config> Pallet<T> {
	fn do_deposit(who: &T::AccountId, underlying_asset_id: CurrencyId, underlying_amount: Balance) -> TokensResult {
		ensure!(
			<LiquidityPools<T>>::is_enabled_underlying_asset_id(underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		ensure!(underlying_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);

		ensure!(
			underlying_amount <= T::MultiCurrency::free_balance(underlying_asset_id, &who),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		// Fail if deposit not allowed
		ensure!(
			<Controller<T>>::is_operation_allowed(underlying_asset_id, Operation::Deposit),
			Error::<T>::OperationPaused
		);

		let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)
			.map_err(|_| Error::<T>::NotValidUnderlyingAssetId)?;

		let wrapped_amount = <LiquidityPools<T>>::convert_to_wrapped(underlying_asset_id, underlying_amount)
			.map_err(|_| Error::<T>::NumOverflow)?;

		T::MultiCurrency::transfer(
			underlying_asset_id,
			&who,
			&T::ManagerLiquidityPools::pools_account_id(),
			underlying_amount,
		)?;

		T::MultiCurrency::deposit(wrapped_id, &who, wrapped_amount)?;

		Ok((underlying_amount, wrapped_id, wrapped_amount))
	}

	fn do_redeem(
		who: &T::AccountId,
		underlying_asset_id: CurrencyId,
		mut underlying_amount: Balance,
		wrapped_amount: Balance,
		all_assets: bool,
	) -> TokensResult {
		ensure!(
			<LiquidityPools<T>>::is_enabled_underlying_asset_id(underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		let wrapped_id = <LiquidityPools<T>>::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)
			.map_err(|_| Error::<T>::NotValidUnderlyingAssetId)?;

		let wrapped_amount = match (underlying_amount, wrapped_amount, all_assets) {
			(0, 0, true) => {
				let total_wrapped_amount = T::MultiCurrency::free_balance(wrapped_id, &who);
				ensure!(!total_wrapped_amount.is_zero(), Error::<T>::NumberOfWrappedTokensIsZero);
				underlying_amount = <LiquidityPools<T>>::convert_from_wrapped(wrapped_id, total_wrapped_amount)
					.map_err(|_| Error::<T>::NumOverflow)?;
				total_wrapped_amount
			}
			(_, 0, false) => {
				ensure!(underlying_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
				<LiquidityPools<T>>::convert_to_wrapped(underlying_asset_id, underlying_amount)
					.map_err(|_| Error::<T>::NumOverflow)?
			}
			_ => {
				ensure!(wrapped_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
				underlying_amount = <LiquidityPools<T>>::convert_from_wrapped(wrapped_id, wrapped_amount)
					.map_err(|_| Error::<T>::NumOverflow)?;
				wrapped_amount
			}
		};

		ensure!(
			underlying_amount <= T::ManagerLiquidityPools::get_pool_available_liquidity(underlying_asset_id),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		ensure!(
			wrapped_amount <= T::MultiCurrency::free_balance(wrapped_id, &who),
			Error::<T>::NotEnoughWrappedTokens
		);

		// Fail if redeem not allowed
		ensure!(
			<Controller<T>>::is_operation_allowed(underlying_asset_id, Operation::Redeem),
			Error::<T>::OperationPaused
		);
		<Controller<T>>::redeem_allowed(underlying_asset_id, &who, wrapped_amount)
			.map_err(|_| Error::<T>::RedeemControllerRejection)?;

		T::MultiCurrency::withdraw(wrapped_id, &who, wrapped_amount)?;

		T::MultiCurrency::transfer(
			underlying_asset_id,
			&T::ManagerLiquidityPools::pools_account_id(),
			&who,
			underlying_amount,
		)?;

		Ok((underlying_amount, wrapped_id, wrapped_amount))
	}

	/// Users borrow assets from the protocol to their own address
	///
	/// - `who`: the address of the user who borrows.
	/// - `underlying_asset_id`: the currency ID of the underlying asset to borrow.
	/// - `underlying_amount`: the amount of the underlying asset to borrow.
	fn do_borrow(who: &T::AccountId, underlying_asset_id: CurrencyId, borrow_amount: Balance) -> DispatchResult {
		ensure!(
			<LiquidityPools<T>>::is_enabled_underlying_asset_id(underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		let pool_available_liquidity = T::ManagerLiquidityPools::get_pool_available_liquidity(underlying_asset_id);

		// Raise an error if protocol has insufficient underlying cash.
		ensure!(
			borrow_amount <= pool_available_liquidity,
			Error::<T>::NotEnoughLiquidityAvailable
		);

		ensure!(borrow_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		// Fail if borrow not allowed.
		ensure!(
			<Controller<T>>::is_operation_allowed(underlying_asset_id, Operation::Borrow),
			Error::<T>::OperationPaused
		);
		<Controller<T>>::borrow_allowed(underlying_asset_id, &who, borrow_amount)?;

		// Fetch the amount the borrower owes, with accumulated interest.
		let account_borrows =
			<Controller<T>>::borrow_balance_stored(&who, underlying_asset_id).map_err(|_| Error::<T>::NumOverflow)?;

		<LiquidityPools<T>>::update_state_on_borrow(&who, underlying_asset_id, borrow_amount, account_borrows)
			.map_err(|_| Error::<T>::NumOverflow)?;

		// Transfer the borrow_amount from the protocol account to the borrower's account.
		T::MultiCurrency::transfer(
			underlying_asset_id,
			&T::ManagerLiquidityPools::pools_account_id(),
			&who,
			borrow_amount,
		)?;

		Ok(())
	}

	/// Sender repays their own borrow
	///
	/// - `who`: the account paying off the borrow.
	/// - `borrower`: the account with the debt being payed off.
	/// - `underlying_asset_id`: the currency ID of the underlying asset to repay.
	/// - `repay_amount`: the amount of the underlying asset to repay.
	fn do_repay(
		who: &T::AccountId,
		borrower: &T::AccountId,
		underlying_asset_id: CurrencyId,
		mut repay_amount: Balance,
		all_assets: bool,
	) -> BalanceResult {
		ensure!(
			<LiquidityPools<T>>::is_enabled_underlying_asset_id(underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);
		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;
		repay_amount = Self::do_repay_fresh(who, borrower, underlying_asset_id, repay_amount, all_assets)?;
		Ok(repay_amount)
	}

	/// Borrows are repaid by another user (possibly the borrower).
	///
	/// - `who`: the account paying off the borrow.
	/// - `borrower`: the account with the debt being payed off.
	/// - `underlying_asset_id`: the currency ID of the underlying asset to repay.
	/// - `repay_amount`: the amount of the underlying asset to repay.
	pub fn do_repay_fresh(
		who: &T::AccountId,
		borrower: &T::AccountId,
		underlying_asset_id: CurrencyId,
		mut repay_amount: Balance,
		all_assets: bool,
	) -> BalanceResult {
		if !all_assets {
			ensure!(repay_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
		}

		// Fail if repay_borrow not allowed
		ensure!(
			<Controller<T>>::is_operation_allowed(underlying_asset_id, Operation::Repay),
			Error::<T>::OperationPaused
		);

		// Fetch the amount the borrower owes, with accumulated interest
		let account_borrows = <Controller<T>>::borrow_balance_stored(&borrower, underlying_asset_id)
			.map_err(|_| Error::<T>::NumOverflow)?;

		repay_amount = match repay_amount.cmp(&Balance::zero()) {
			Ordering::Equal => account_borrows,
			_ => repay_amount,
		};

		ensure!(
			repay_amount <= T::MultiCurrency::free_balance(underlying_asset_id, &who),
			Error::<T>::NotEnoughUnderlyingsAssets
		);

		<LiquidityPools<T>>::update_state_on_repay(&borrower, underlying_asset_id, repay_amount, account_borrows)
			.map_err(|_| Error::<T>::RepayAmountToBig)?;

		// Transfer the repay_amount from the borrower's account to the protocol account.
		T::MultiCurrency::transfer(
			underlying_asset_id,
			&who,
			&T::ManagerLiquidityPools::pools_account_id(),
			repay_amount,
		)?;

		Ok(repay_amount)
	}

	/// Sender transfers their tokens to other account
	///
	/// - `who`: the account transferring tokens.
	/// - `receiver`: the account that will receive tokens.
	/// - `wrapped_id`: the currency ID of the wrapped asset to transfer.
	/// - `transfer_amount`: the amount of the wrapped asset to transfer.
	fn do_transfer(
		who: &T::AccountId,
		receiver: &T::AccountId,
		wrapped_id: CurrencyId,
		transfer_amount: Balance,
	) -> DispatchResult {
		ensure!(transfer_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
		ensure!(who != receiver, Error::<T>::CannotTransferToSelf);

		// Fail if invalid token id
		let underlying_asset_id = <LiquidityPools<T>>::get_underlying_asset_id_by_wrapped_id(&wrapped_id)
			.map_err(|_| Error::<T>::NotValidWrappedTokenId)?;

		// Fail if transfer is not allowed
		ensure!(
			<Controller<T>>::is_operation_allowed(underlying_asset_id, Operation::Transfer),
			Error::<T>::OperationPaused
		);

		// Fail if transfer_amount is not available for redeem
		<Controller<T>>::redeem_allowed(underlying_asset_id, &who, transfer_amount)
			.map_err(|_| Error::<T>::RedeemControllerRejection)?;

		// Fail if not enough free balance
		ensure!(
			transfer_amount <= T::MultiCurrency::free_balance(wrapped_id, &who),
			Error::<T>::NotEnoughWrappedTokens
		);

		// Transfer the transfer_amount from one account to another
		T::MultiCurrency::transfer(wrapped_id, &who, &receiver, transfer_amount)?;

		Ok(())
	}
}
