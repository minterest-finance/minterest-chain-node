#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system, ensure_signed};
use minterest_primitives::{Balance, CurrencyId};
use orml_traits::MultiCurrency;
use orml_utilities::with_transaction_result;
use pallet_traits::Borrowing;
use sp_runtime::{traits::Zero, DispatchError, DispatchResult};
use sp_std::cmp::Ordering;
use sp_std::{prelude::Vec, result};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: liquidity_pools::Trait + controller::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// Basic borrowing functions
	type Borrowing: Borrowing<Self::AccountId>;
}

type LiquidityPools<T> = liquidity_pools::Module<T>;
type Controller<T> = controller::Module<T>;

decl_storage! {
	trait Store for Module<T: Trait> as MinterestProtocol {
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
	{
		/// Underlying assets added to pool and wrapped tokens minted: \[who, underlying_asset_id, underlying_amount, wrapped_currency_id, wrapped_amount\]
		Deposited(AccountId, CurrencyId, Balance, CurrencyId, Balance),

		/// Underlying assets and wrapped tokens redeemed: \[who, underlying_asset_id, underlying_amount, wrapped_currency_id, wrapped_amount\]
		Redeemed(AccountId, CurrencyId, Balance, CurrencyId, Balance),

		/// Borrowed a specific amount of the pool currency: \[who, underlying_asset_id, the_amount_to_be_borrowed\]
		Borrowed(AccountId, CurrencyId, Balance),

		/// Repaid a borrow on the specific pool, for the specified amount: \[who, underlying_asset_id, the_amount_repaid\]
		Repaid(AccountId, CurrencyId, Balance),

		/// The user allowed the assets in the pool to be used as collateral: \[who, pool_id\]
		PoolEnabledAsCollateral(AccountId, CurrencyId),

		/// The user denies use the assets in pool as collateral: \[who, pool_id\]
		PoolDisabledCollateral(AccountId, CurrencyId),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
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

		/// Borrow was blocked due to Controller rejection.
		BorrowControllerRejection,

		/// Repay was blocked due to Controller rejection.
		RepayBorrowControllerRejection,

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
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		const UnderlyingAssetId: Vec<CurrencyId> = T::UnderlyingAssetId::get();

		/// Transfers an asset into the protocol. The user receives a quantity of mTokens equal
		/// to the underlying tokens supplied, divided by the current Exchange Rate.
		///
		/// - `underlying_asset_id`: CurrencyId of underlying assets to be transferred into the protocol.
		/// - `underlying_amount`: The amount of the asset to be supplied, in units of the underlying asset.
		#[weight = 10_000]
		pub fn deposit_underlying(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] underlying_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let (_, wrapped_id, wrapped_amount) = Self::do_deposit(&who, underlying_asset_id, underlying_amount)?;
				Self::deposit_event(RawEvent::Deposited(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Converts ALL mTokens into a specified quantity of the underlying asset, and returns them
		/// to the user. The amount of underlying tokens received is equal to the quantity of
		/// mTokens redeemed, multiplied by the current Exchange Rate.
		///
		/// - `underlying_asset_id`: CurrencyId of underlying assets to be redeemed.
		#[weight = 10_000]
		pub fn redeem(
			origin,
			underlying_asset_id: CurrencyId,
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let (underlying_amount, wrapped_id, wrapped_amount) = Self::do_redeem(&who, underlying_asset_id, Balance::zero(), Balance::zero(), true)?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Converts mTokens into a specified quantity of the underlying asset, and returns them to
		/// the user. The amount of mTokens redeemed is equal to the quantity of underlying tokens
		/// received, divided by the current Exchange Rate.
		///
		/// - `underlying_asset_id`: CurrencyId of underlying assets to be redeemed.
		/// - `underlying_amount`: The number of underlying assets to be redeemed.
		#[weight = 10_000]
		pub fn redeem_underlying(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] underlying_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let (_, wrapped_id, wrapped_amount) = Self::do_redeem(&who, underlying_asset_id, underlying_amount, Balance::zero(), false)?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Converts a specified quantity of mTokens into the underlying asset, and returns them to the user.
		/// The amount of underlying tokens received is equal to the quantity of mTokens redeemed,
		/// multiplied by the current Exchange Rate.
		///
		/// - `wrapped_id`: CurrencyId of mTokens to be redeemed.
		/// - `wrapped_amount`: The number of mTokens to be redeemed.
		#[weight = 10_000]
		pub fn redeem_wrapped(origin, wrapped_id: CurrencyId, #[compact] wrapped_amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let underlying_asset_id = <Controller<T>>::get_underlying_asset_id_by_wrapped_id(&wrapped_id).map_err(|_| Error::<T>::NotValidWrappedTokenId)?;
				let (underlying_amount, wrapped_id, _) = Self::do_redeem(&who, underlying_asset_id, Balance::zero(), wrapped_amount, false)?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Borrowing a specific amount of the pool currency, provided that the borrower already
		/// deposited enough collateral.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be borrowed.
		/// - `underlying_amount`: The amount of the underlying asset to be borrowed.
		#[weight = 10_000]
		pub fn borrow(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] borrow_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_borrow(&who, underlying_asset_id, borrow_amount)?;
				Self::deposit_event(RawEvent::Borrowed(who, underlying_asset_id, borrow_amount));
				Ok(())
			})?;
		}

		/// Repays a borrow on the specific pool, for the specified amount.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be repaid.
		/// - `repay_amount`: The amount of the underlying asset to be repaid.
		#[weight = 10_000]
		pub fn repay(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] repay_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_repay(&who, &who, underlying_asset_id, repay_amount, false)?;
				Self::deposit_event(RawEvent::Repaid(who, underlying_asset_id, repay_amount));
				Ok(())
			})?;
		}

		/// Repays a borrow on the specific pool, for the all amount.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be repaid.
		#[weight = 10_000]
		pub fn repay_all(origin, underlying_asset_id: CurrencyId) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let repay_amount = Self::do_repay(&who, &who, underlying_asset_id, Balance::zero(), true)?;
				Self::deposit_event(RawEvent::Repaid(who, underlying_asset_id, repay_amount));
				Ok(())
			})?;
		}

		/// Transfers an asset into the protocol, reducing the target user's borrow balance.
		///
		/// - `underlying_asset_id`: The currency ID of the underlying asset to be repaid.
		/// - `borrower`: The account which borrowed the asset to be repaid.
		/// - `repay_amount`: The amount of the underlying borrowed asset to be repaid.
		#[weight = 10_000]
		pub fn repay_on_behalf(origin, underlying_asset_id: CurrencyId, borrower: T::AccountId, repay_amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let repay_amount = Self::do_repay(&who, &borrower, underlying_asset_id, repay_amount, false)?;
				Self::deposit_event(RawEvent::Repaid(who, underlying_asset_id, repay_amount));
				Ok(())
			})?
		}

		/// Sender allowed the assets in the pool to be used as collateral.
		#[weight = 10_000]
		pub fn enable_as_collateral(origin, pool_id: CurrencyId) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				T::UnderlyingAssetId::get().contains(&pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			ensure!(!<LiquidityPools<T>>::check_user_available_collateral(&sender, pool_id), Error::<T>::AlreadyCollateral);

			// If user does not have assets in the pool, then he cannot enable as collateral the pool.
			let wrapped_id = <Controller<T>>::get_wrapped_id_by_underlying_asset_id(&pool_id)?;
			let user_wrapped_balance = T::MultiCurrency::free_balance(wrapped_id, &sender);
			ensure!(user_wrapped_balance > 0, Error::<T>::CanotBeEnabledAsCollateral);

			<LiquidityPools<T>>::enable_as_collateral_internal(&sender, pool_id)?;
			Self::deposit_event(RawEvent::PoolEnabledAsCollateral(sender, pool_id));
			Ok(())
		}

		/// Sender has denies use the assets in pool as collateral.
		#[weight = 10_000]
		pub fn disable_collateral(origin, pool_id: CurrencyId) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				T::UnderlyingAssetId::get().contains(&pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			ensure!(<LiquidityPools<T>>::check_user_available_collateral(&sender, pool_id), Error::<T>::AlreadyDisabledCollateral);

			let wrapped_id = <Controller<T>>::get_wrapped_id_by_underlying_asset_id(&pool_id)?;
			let user_balance_wrapped_tokens = T::MultiCurrency::free_balance(wrapped_id, &sender);
			let user_balance_disabled_asset = <Controller<T>>::convert_from_wrapped(wrapped_id, user_balance_wrapped_tokens)?;

			// Check if the user will have enough collateral if he removes one of the collaterals.
			let (_, shortfall) = <Controller<T>>::get_hypothetical_account_liquidity(&sender, pool_id, user_balance_disabled_asset, 0)?;
			ensure!(!(shortfall > 0), Error::<T>::CanotBeDisabledAsCollateral);

			<LiquidityPools<T>>::disable_collateral_internal(&sender, pool_id)?;
			Self::deposit_event(RawEvent::PoolDisabledCollateral(sender, pool_id));
			Ok(())
		}
	}
}

type TokensResult = result::Result<(Balance, CurrencyId, Balance), DispatchError>;
type BalanceResult = result::Result<Balance, DispatchError>;

// Dispatchable calls implementation
impl<T: Trait> Module<T> {
	fn do_deposit(who: &T::AccountId, underlying_asset_id: CurrencyId, underlying_amount: Balance) -> TokensResult {
		ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
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

		let wrapped_id = <Controller<T>>::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)
			.map_err(|_| Error::<T>::NotValidUnderlyingAssetId)?;

		let wrapped_amount = <Controller<T>>::convert_to_wrapped(underlying_asset_id, underlying_amount)
			.map_err(|_| Error::<T>::NumOverflow)?;

		T::MultiCurrency::transfer(
			underlying_asset_id,
			&who,
			&<LiquidityPools<T>>::pools_account_id(),
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
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		let wrapped_id = <Controller<T>>::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)
			.map_err(|_| Error::<T>::NotValidUnderlyingAssetId)?;

		let wrapped_amount = match (underlying_amount, wrapped_amount, all_assets) {
			(0, 0, true) => {
				let total_wrapped_amount = T::MultiCurrency::free_balance(wrapped_id, &who);
				ensure!(!total_wrapped_amount.is_zero(), Error::<T>::NumberOfWrappedTokensIsZero);
				underlying_amount = <Controller<T>>::convert_from_wrapped(wrapped_id, total_wrapped_amount)
					.map_err(|_| Error::<T>::NumOverflow)?;
				total_wrapped_amount
			}
			(_, 0, false) => {
				ensure!(underlying_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
				<Controller<T>>::convert_to_wrapped(underlying_asset_id, underlying_amount)
					.map_err(|_| Error::<T>::NumOverflow)?
			}
			_ => {
				ensure!(wrapped_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
				underlying_amount = <Controller<T>>::convert_from_wrapped(wrapped_id, wrapped_amount)
					.map_err(|_| Error::<T>::NumOverflow)?;
				wrapped_amount
			}
		};

		ensure!(
			underlying_amount <= <LiquidityPools<T>>::get_pool_available_liquidity(underlying_asset_id),
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
			&<LiquidityPools<T>>::pools_account_id(),
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
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		let pool_available_liquidity = <LiquidityPools<T>>::get_pool_available_liquidity(underlying_asset_id);

		// Raise an error if protocol has insufficient underlying cash
		ensure!(
			borrow_amount <= pool_available_liquidity,
			Error::<T>::NotEnoughLiquidityAvailable
		);

		ensure!(borrow_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		// Fail if borrow not allowed
		ensure!(
			<Controller<T>>::is_operation_allowed(underlying_asset_id, Operation::Borrow),
			Error::<T>::OperationPaused
		);
		<Controller<T>>::borrow_allowed(underlying_asset_id, &who, borrow_amount)
			.map_err(|_| Error::<T>::BorrowControllerRejection)?;

		// Fetch the amount the borrower owes, with accumulated interest
		let account_borrows =
			<Controller<T>>::borrow_balance_stored(&who, underlying_asset_id).map_err(|_| Error::<T>::NumOverflow)?;

		<LiquidityPools<T>>::update_state_on_borrow(&who, underlying_asset_id, borrow_amount, account_borrows)
			.map_err(|_| Error::<T>::NumOverflow)?;

		// Transfer the borrow_amount from the protocol account to the borrower's account.
		T::MultiCurrency::transfer(
			underlying_asset_id,
			&<LiquidityPools<T>>::pools_account_id(),
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
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		if !all_assets {
			ensure!(repay_amount > Balance::zero(), Error::<T>::ZeroBalanceTransaction);
		}

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

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
			&<LiquidityPools<T>>::pools_account_id(),
			repay_amount,
		)?;

		Ok(repay_amount)
	}
}
