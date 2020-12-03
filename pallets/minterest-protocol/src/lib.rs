#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_event, decl_module, decl_storage, decl_error, ensure,
    traits::{Get},
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{MultiCurrency};
use orml_utilities::with_transaction_result;
use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::DispatchError;
use sp_std::{result, prelude::Vec};
use pallet_traits::{LiquidityPools};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod mock;

pub trait Trait: m_tokens::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Wrapped currency IDs.
    type UnderlyingAssetId: Get<Vec<CurrencyId>>;

    /// The Liquidity pools
    type LiqudityPools: LiquidityPools;
}

decl_storage! {
	trait Store for Module<T: Trait> as MinterestProtocol {
	}
}

decl_event!(
	pub enum Event<T> where
	    <T as system::Trait>::AccountId,
    {
	    /// Underlying assets added to pool and wrapped tokens minted: \[who, wrapped_currency_id, liquidity_amount, wrapped_amount\]
		Minted(AccountId, CurrencyId, Balance, Balance),

		/// Underlying assets and wrapped tokens redeemed: \[who, wrapped_currency_id, liquidity_amount, wrapped_amount\]
		Redeemed(AccountId, CurrencyId, Balance, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
        /// The currency is not enabled in wrapped protocol.
		NotValidUnderlyingAssetId,

		/// Insufficient funds in the user account
		NotEnoughUnderlyingAssets,

		/// Insufficient funds in the user account
		NotEnoughWrappedTokens,

		/// Insufficient liquidity in pool for minting.
		InsufficientLiquidityInPool,

		/// Insufficient amount of collateral locked in protocol.
		InsufficientLockedCollateral,

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
        fn mint(
            origin,
            underlying_asset_id: CurrencyId,
            #[compact] liquidity_amount: Balance
        ) {
            with_transaction_result(|| {
                let who = ensure_signed(origin)?;
                let wrapped_amount = Self::do_mint(&who, underlying_asset_id, liquidity_amount)?;
                Self::deposit_event(RawEvent::Minted(who, underlying_asset_id, liquidity_amount, wrapped_amount));
                Ok(())
            })?;
        }

        /// Withdraw underlying assets from pool and burn wrapped tokens.
        #[weight = 10_000]
        fn burn(
            origin,
            underlying_asset_id: CurrencyId,
            #[compact] liquidity_amount: Balance
        ) {
            with_transaction_result(|| {
                let who = ensure_signed(origin)?;
                let wrapped_amount = Self::do_withdraw(&who, underlying_asset_id, liquidity_amount)?;
                Self::deposit_event(RawEvent::Redeemed(who, underlying_asset_id, liquidity_amount, wrapped_amount));
                Ok(())
            })?;
        }
	}
}

type BalanceResult = result::Result<Balance, DispatchError>;

// Dispatchable calls implementation
impl<T: Trait> Module<T> {
    fn do_mint(
        who: &T::AccountId,
        underlying_asset_id: CurrencyId,
        liquidity_amount: Balance,
    ) -> BalanceResult {
        ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);
        ensure!(
            liquidity_amount <= T::MultiCurrency::free_balance(underlying_asset_id, &who),
            Error::<T>::NotEnoughUnderlyingAssets
        );

        // wrapped_amount = liquidity_amount
        let wrapped_value = liquidity_amount;

        let currency_id = Self::get_currency_id_by_underlying_asset_id(&underlying_asset_id)?;

        T::MultiCurrency::withdraw(underlying_asset_id, &who, liquidity_amount)?;

        T::LiqudityPools::add_liquidity(&underlying_asset_id, &liquidity_amount)
            .map_err(|_| Error::<T>::InsufficientLiquidityInPool)?;

        T::MultiCurrency::deposit(currency_id, &who, wrapped_value)?;

        Ok(wrapped_value)
    }

    fn do_withdraw(
        who: &T::AccountId,
        underlying_asset_id: CurrencyId,
        liquidity_amount: Balance,
    ) -> BalanceResult {
        ensure!(
			T::UnderlyingAssetId::get().contains(&underlying_asset_id),
			Error::<T>::NotValidUnderlyingAssetId
		);

        // wrapped_amount = liquidity_amount
        let required_wrapped_value = liquidity_amount;

        let currency_id = Self::get_currency_id_by_underlying_asset_id(&underlying_asset_id)?;

        ensure!(
            required_wrapped_value <= T::MultiCurrency::free_balance(currency_id, &who),
            Error::<T>::NotEnoughWrappedTokens
        );

        T::MultiCurrency::withdraw(currency_id, &who, required_wrapped_value)?;

        T::LiqudityPools::withdraw_liquidity(&underlying_asset_id, &liquidity_amount)
            .map_err(|_| Error::<T>::InsufficientLockedCollateral)?;

        T::MultiCurrency::deposit(underlying_asset_id, &who, liquidity_amount)?;

        Ok(required_wrapped_value)
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
