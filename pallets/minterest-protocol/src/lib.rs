#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_event, decl_module, decl_storage, decl_error, ensure,
    traits::{Get},
};
use frame_system::{self as system, ensure_signed};
use orml_utilities::with_transaction_result;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::{DispatchResult};
use sp_std::{result, prelude::Vec};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod mock;

pub trait Trait: m_tokens::Trait + liquidity_pools::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Wrapped currency IDs.
    type UnderlyingAssetId: Get<Vec<CurrencyId>>;
}

type MTokens<T> = m_tokens::Module<T>;
type LiquidityPools<T> = liquidity_pools::Module<T>;

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

	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
        /// The currency is not enabled in wrapped protocol.
		NotValidUnderlyingAssetId,

		/// There is not enough liquidity available to redeem
		NotEnoughLiquidityAvailable,

		/// Insufficient funds in the user account
		NotEnoughWrappedTokens,

		/// PoolNotFound or NotEnoughBalance or BalanceOverflowed
		InternalReserveError,
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
	}
}

// Dispatchable calls implementation
impl<T: Trait> Module<T> {
    fn do_deposit(
        who: &T::AccountId,
        underlying_asset_id: CurrencyId,
        amount: Balance,
    ) -> DispatchResult {
        ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);
        ensure!(
            amount <= <MTokens<T>>::free_balance(underlying_asset_id, &who),
            Error::<T>::NotEnoughLiquidityAvailable
        );

        let currency_id = Self::get_currency_id_by_underlying_asset_id(&underlying_asset_id)?;

        <MTokens<T>>::withdraw(underlying_asset_id, &who, amount)?;

        <LiquidityPools<T>>::update_state_on_deposit(amount, underlying_asset_id)
            .map_err(|_| Error::<T>::InternalReserveError)?;

        <MTokens<T>>::deposit(currency_id, &who, amount)?;

        Ok(())
    }

    fn do_redeem(
        who: &T::AccountId,
        underlying_asset_id: CurrencyId,
        amount: Balance,
    ) -> DispatchResult {
        ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

        ensure!(
            amount <= <LiquidityPools<T>>::get_reserve_available_liquidity(underlying_asset_id),
            Error::<T>::NotEnoughLiquidityAvailable
        );

        let currency_id = Self::get_currency_id_by_underlying_asset_id(&underlying_asset_id)?;

        ensure!(
            amount <= <MTokens<T>>::free_balance(currency_id, &who),
            Error::<T>::NotEnoughWrappedTokens
        );

        <MTokens<T>>::withdraw(currency_id, &who, amount)?;

        <LiquidityPools<T>>::update_state_on_redeem(amount, underlying_asset_id)
            .map_err(|_| Error::<T>::InternalReserveError)?;

        <MTokens<T>>::deposit(underlying_asset_id, &who, amount)?;

        Ok(())
    }
}

// Private methods
impl<T: Trait> Module<T> {
    fn get_currency_id_by_underlying_asset_id(
        asset_id: &CurrencyId
    ) -> result::Result<CurrencyId, Error<T>> {
        match asset_id {
            CurrencyId::DOT => Ok(CurrencyId::MDOT),
            CurrencyId::KSM => Ok(CurrencyId::MKSM),
            CurrencyId::BTC => Ok(CurrencyId::MBTC),
            CurrencyId::ETH => Ok(CurrencyId::METH),
            _ => Err(Error::<T>::NotValidUnderlyingAssetId),
        }
    }
}
