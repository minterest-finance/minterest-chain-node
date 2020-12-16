#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system, ensure_signed};
use minterest_primitives::{Balance, CurrencyId};
use orml_utilities::with_transaction_result;
use pallet_traits::Borrowing;
use sp_runtime::{DispatchResult, FixedPointNumber};
use sp_std::{prelude::Vec, result};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait Trait: m_tokens::Trait + liquidity_pools::Trait + controller::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// Wrapped currency IDs.
	type UnderlyingAssetId: Get<Vec<CurrencyId>>;

	/// Basic borrowing functions
	type Borrowing: Borrowing<Self::AccountId>;
}

type MTokens<T> = m_tokens::Module<T>;
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
		/// Underlying assets added to pool and wrapped tokens minted: \[who, wrapped_currency_id, liquidity_amount\]
		Deposited(AccountId, CurrencyId, Balance),

		/// Underlying assets and wrapped tokens redeemed: \[who, wrapped_currency_id, liquidity_amount\]
		Redeemed(AccountId, CurrencyId, Balance),

		/// Borrowed a specific amount of the reserve currency: \[who, underlying_asset_id, the_amount_to_be_deposited\]
		Borrowed(AccountId, CurrencyId, Balance),

		/// Repaid a borrow on the specific reserve, for the specified amount: \[who, underlying_asset_id, the_amount_repaid\]
		Repaid(AccountId, CurrencyId, Balance),

	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// The currency is not enabled in wrapped protocol.
		NotValidUnderlyingAssetId,

		/// There is not enough liquidity available in the reserve.
		NotEnoughLiquidityAvailable,

		/// Insufficient funds in the user account.
		NotEnoughWrappedTokens,

		/// PoolNotFound or NotEnoughBalance or BalanceOverflowed.
		InternalReserveError,

		/// Number overflow in calculation.
		NumOverflow,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		const UnderlyingAssetId: Vec<CurrencyId> = T::UnderlyingAssetId::get();

		/// Add Underlying Assets to pool and mint wrapped tokens.
		#[weight = 10_000]
		pub fn deposit_underlying(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] liquidity_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_deposit(&who, underlying_asset_id, liquidity_amount)?;
				Self::deposit_event(RawEvent::Deposited(who, underlying_asset_id, liquidity_amount));
				Ok(())
			})?;
		}

		/// Withdraw underlying assets from pool and burn wrapped tokens.
		#[weight = 10_000]
		pub fn redeem_underlying(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] liquidity_amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_redeem(&who, underlying_asset_id, liquidity_amount)?;
				Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, liquidity_amount));
				Ok(())
			})?;
		}

		/// Borrowing a specific amount of the reserve currency, provided that the borrower already deposited enough collateral.
		#[weight = 10_000]
		pub fn borrow(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_borrow(&who, underlying_asset_id, amount)?;
				Self::deposit_event(RawEvent::Borrowed(who, underlying_asset_id, amount));
				Ok(())
			})?;
		}

		/// Repays a borrow on the specific reserve, for the specified amount.
		#[weight = 10_000]
		pub fn repay(
			origin,
			underlying_asset_id: CurrencyId,
			#[compact] amount: Balance
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_repay(&who, underlying_asset_id, amount)?;
				Self::deposit_event(RawEvent::Repaid(who, underlying_asset_id, amount));
				Ok(())
			})?;
		}
	}
}

// Dispatchable calls implementation
impl<T: Trait> Module<T> {
	fn do_deposit(who: &T::AccountId, underlying_asset_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);
		ensure!(
			amount <= <MTokens<T>>::free_balance(underlying_asset_id, &who),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		let wrapped_id = Self::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)?;

		let liquidity_rate = <Controller<T>>::calculate_liquidity_rate(underlying_asset_id)?;

		// wrapped = underlying / liquidity_rate
		let wrapped_amount = amount
			.checked_div(liquidity_rate.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		<MTokens<T>>::withdraw(underlying_asset_id, &who, amount)?;

		<LiquidityPools<T>>::update_state_on_deposit(amount, underlying_asset_id)
			.map_err(|_| Error::<T>::InternalReserveError)?;

		<MTokens<T>>::deposit(wrapped_id, &who, wrapped_amount)?;

		Ok(())
	}

	fn do_redeem(who: &T::AccountId, underlying_asset_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		ensure!(
			amount <= <LiquidityPools<T>>::get_reserve_available_liquidity(underlying_asset_id),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		let wrapped_id = Self::get_wrapped_id_by_underlying_asset_id(&underlying_asset_id)?;

		let liquidity_rate = <Controller<T>>::calculate_liquidity_rate(underlying_asset_id)?;

		// wrapped = underlying / liquidity_rate
		let wrapped_amount = amount
			.checked_div(liquidity_rate.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		ensure!(
			wrapped_amount <= <MTokens<T>>::free_balance(wrapped_id, &who),
			Error::<T>::NotEnoughWrappedTokens
		);

		<MTokens<T>>::withdraw(wrapped_id, &who, wrapped_amount)?;

		<LiquidityPools<T>>::update_state_on_redeem(amount, underlying_asset_id)
			.map_err(|_| Error::<T>::InternalReserveError)?;

		<MTokens<T>>::deposit(underlying_asset_id, &who, amount)?;

		Ok(())
	}

	fn do_borrow(who: &T::AccountId, underlying_asset_id: CurrencyId, amount: Balance) -> DispatchResult {
		ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

		ensure!(
			amount <= <LiquidityPools<T>>::get_reserve_available_liquidity(underlying_asset_id),
			Error::<T>::NotEnoughLiquidityAvailable
		);

		//TODO rewrite after implementing the function in the controller.
		// This function should return current information about the user and his balances.
		<Controller<T>>::calculate_user_global_data(who.clone())?;

		//TODO rewrite after implementing the function in the controller.
		// This function should return the amount of collateral needed in dollars.
		<Controller<T>>::calculate_total_available_collateral(amount, underlying_asset_id)?;

		<LiquidityPools<T>>::update_state_on_borrow(underlying_asset_id, amount, who)
			.map_err(|_| Error::<T>::InternalReserveError)?;

		<MTokens<T>>::deposit(underlying_asset_id, who, amount)?;

		Ok(())
	}

	fn do_repay(_who: &T::AccountId, _underlying_asset_id: CurrencyId, _amount: Balance) -> DispatchResult {
		Ok(())
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
}
