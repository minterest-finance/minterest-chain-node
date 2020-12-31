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

	/// Wrapped currency IDs.
	type UnderlyingAssetId: Get<Vec<CurrencyId>>;

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

		/// Insufficient underlying assets in the user account.
		NotEnoughUnderlyingsAssets,

		/// PoolNotFound or NotEnoughBalance or BalanceOverflowed.
		InternalPoolError,

		/// Number overflow in calculation.
		NumOverflow,

		/// The block number in the pool is equal to the current block number.
		PoolNotFresh,

		/// An internal failure occurred in the execution of the Accrue Interest function.
		AccrueInterestFailed,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		const UnderlyingAssetId: Vec<CurrencyId> = T::UnderlyingAssetId::get();

		/// Sender supplies assets into the pool and receives mTokens in exchange.
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

		/// Sender redeems mTokens in exchange for the underlying assets.
		#[weight = 10_000]
		pub fn redeem(
			origin,
			underlying_asset_id: CurrencyId,
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let (underlying_amount, wrapped_id, wrapped_amount) = Self::do_redeem(&who, underlying_asset_id, Balance::zero(), Balance::zero())?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Sender redeems mTokens in exchange for a specified amount of underlying assets.
		#[weight = 10_000]
		pub fn redeem_underlying(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] underlying_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let (_, wrapped_id, wrapped_amount) = Self::do_redeem(&who, underlying_asset_id, underlying_amount, Balance::zero())?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Sender redeems a specified amount of mTokens in exchange for the underlying assets.
		#[weight = 10_000]
		pub fn redeem_wrapped(origin, wrapped_id: CurrencyId, #[compact] wrapped_amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let underlying_asset_id = Self::get_underlying_asset_id_by_wrapped_id(&wrapped_id)?;
				let (underlying_amount, wrapped_id, _) = Self::do_redeem(&who, underlying_asset_id, Balance::zero(), wrapped_amount)?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, underlying_amount, wrapped_id, wrapped_amount));
				Ok(())
			})?;
		}

		/// Borrowing a specific amount of the pool currency, provided that the borrower already deposited enough collateral.
		///
		/// - `underlying_asset_id`: the currency ID of the underlying asset to borrow.
		/// - `underlying_amount`: the amount of the underlying asset to borrow.
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
		/// - `underlying_asset_id`: the currency ID of the underlying asset to repay.
		/// - `underlying_amount`: the amount of the underlying asset to repay.
		#[weight = 10_000]
		pub fn repay(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] repay_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_repay(&who, underlying_asset_id, repay_amount)?;
				Self::deposit_event(RawEvent::Repaid(who, underlying_asset_id, repay_amount));
				Ok(())
			})?;
		}

		/// Repays a borrow on the specific pool, for the all amount.
		///
		/// - `underlying_asset_id`: the currency ID of the underlying asset to repay.
		/// - `underlying_amount`: the amount of the underlying asset to repay.
		#[weight = 10_000]
		pub fn repay_all(origin, underlying_asset_id: CurrencyId) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let repay_amount = Self::do_repay(&who, underlying_asset_id, Balance::zero())?;
				Self::deposit_event(RawEvent::Repaid(who, underlying_asset_id, repay_amount));
				Ok(())
			})?;
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
		ensure!(
			underlying_amount <= T::MultiCurrency::free_balance(underlying_asset_id, &who),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::InternalPoolError)?;

		let wrapped_id = Self::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)?;

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
	) -> TokensResult {
		ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		ensure!(
			underlying_amount <= <LiquidityPools<T>>::get_pool_available_liquidity(underlying_asset_id),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::InternalPoolError)?;

		let wrapped_id = Self::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)?;

		let wrapped_amount = match (underlying_amount, wrapped_amount) {
			(0, 0) => {
				let total_wrapped_amount = T::MultiCurrency::free_balance(wrapped_id, &who);
				underlying_amount = <Controller<T>>::convert_from_wrapped(wrapped_id, total_wrapped_amount)
					.map_err(|_| Error::<T>::NumOverflow)?;
				total_wrapped_amount
			}
			(_, 0) => <Controller<T>>::convert_to_wrapped(underlying_asset_id, underlying_amount)
				.map_err(|_| Error::<T>::NumOverflow)?,
			_ => {
				underlying_amount = <Controller<T>>::convert_from_wrapped(wrapped_id, wrapped_amount)
					.map_err(|_| Error::<T>::NumOverflow)?;
				wrapped_amount
			}
		};

		ensure!(
			wrapped_amount <= T::MultiCurrency::free_balance(wrapped_id, &who),
			Error::<T>::NotEnoughWrappedTokens
		);

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

		// Raise an error if protocol has insufficient underlying cash
		ensure!(
			borrow_amount <= <LiquidityPools<T>>::get_pool_available_liquidity(underlying_asset_id),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		// Fetch the amount the borrower owes, with accumulated interest
		let account_borrows =
			<Controller<T>>::borrow_balance_stored(&who, underlying_asset_id).map_err(|_| Error::<T>::NumOverflow)?;

		<LiquidityPools<T>>::update_state_on_borrow(&who, underlying_asset_id, borrow_amount, account_borrows)
			.map_err(|_| Error::<T>::InternalPoolError)?;

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
	/// - `underlying_asset_id`: the currency ID of the underlying asset to repay.
	/// - `underlying_amount`: the amount of the underlying asset to repay.
	fn do_repay(who: &T::AccountId, underlying_asset_id: CurrencyId, mut repay_amount: Balance) -> BalanceResult {
		ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		ensure!(
			repay_amount <= T::MultiCurrency::free_balance(underlying_asset_id, &who),
			Error::<T>::NotEnoughUnderlyingsAssets
		);

		<Controller<T>>::accrue_interest_rate(underlying_asset_id).map_err(|_| Error::<T>::AccrueInterestFailed)?;

		// Verify pool's block number equals current block number
		let current_block_number = <frame_system::Module<T>>::block_number();
		let accrual_block_number_previous = <Controller<T>>::controller_dates(underlying_asset_id).timestamp;
		ensure!(
			current_block_number == accrual_block_number_previous,
			Error::<T>::PoolNotFresh
		);

		// Fetch the amount the borrower owes, with accumulated interest
		let account_borrows =
			<Controller<T>>::borrow_balance_stored(&who, underlying_asset_id).map_err(|_| Error::<T>::NumOverflow)?;

		repay_amount = match repay_amount.cmp(&Balance::zero()) {
			Ordering::Equal => account_borrows,
			_ => repay_amount,
		};

		<LiquidityPools<T>>::update_state_on_repay(&who, underlying_asset_id, repay_amount, account_borrows)
			.map_err(|_| Error::<T>::InternalPoolError)?;

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

// Private methods
impl<T: Trait> Module<T> {
	fn get_wrapped_id_by_underlying_asset_id(asset_id: &CurrencyId) -> result::Result<CurrencyId, Error<T>> {
		match asset_id {
			CurrencyId::DOT => Ok(CurrencyId::MDOT),
			CurrencyId::KSM => Ok(CurrencyId::MKSM),
			CurrencyId::BTC => Ok(CurrencyId::MBTC),
			CurrencyId::ETH => Ok(CurrencyId::METH),
			_ => Err(Error::<T>::NotValidUnderlyingAssetId),
		}
	}

	fn get_underlying_asset_id_by_wrapped_id(wrapped_id: &CurrencyId) -> result::Result<CurrencyId, Error<T>> {
		match wrapped_id {
			CurrencyId::MDOT => Ok(CurrencyId::DOT),
			CurrencyId::MKSM => Ok(CurrencyId::KSM),
			CurrencyId::MBTC => Ok(CurrencyId::BTC),
			CurrencyId::METH => Ok(CurrencyId::ETH),
			_ => Err(Error::<T>::NotValidWrappedTokenId),
		}
	}
}
