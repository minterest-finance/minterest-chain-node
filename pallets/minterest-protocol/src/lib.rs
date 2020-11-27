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
	    /// Underlying  assets added to poll and wrapped tokens minted: \[who, wrapped_currency_id, liquidity_amount, wrapped_amount\]
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

		/// Number overflow in calculation.
		NumOverflow,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		const UnderlyingAssetId: Vec<CurrencyId> = T::UnderlyingAssetId::get();

		/// Mint wrapped tokens.
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

        /// Burn wrapped tokens.
        #[weight = 10_000]
        fn burn(origin,
            currency_id: CurrencyId,
            #[compact] amount: Balance
        ) {
            with_transaction_result(|| {
                let who = ensure_signed(origin)?;
                T::MultiCurrency::withdraw(currency_id, &who, amount)?;
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

        T::MultiCurrency::withdraw(underlying_asset_id, &who, liquidity_amount)?;

        let price: u128 = Self::get_price()?;

        // wrapped_amount = liquidity_amount * price
        let wrapped_value = price.checked_mul(liquidity_amount).ok_or(Error::<T>::NumOverflow)?;

        let currency_id = match underlying_asset_id {
            CurrencyId::DOT => CurrencyId::MDOT,
            CurrencyId::KSM => CurrencyId::MKSM,
            CurrencyId::BTC => CurrencyId::MBTC,
            CurrencyId::ETH => CurrencyId::METH,
            _ => unreachable!(),
        };

        T::LiqudityPools::add_liquidity(&underlying_asset_id, &liquidity_amount)?;

        T::MultiCurrency::deposit(currency_id, &who, liquidity_amount)?;

        Ok(wrapped_value)
    }
}

// Private methods
impl<T: Trait> Module<T> {
    // TODO implement the logic for getting the exchange price underlying assets to wrapped tokens
    fn get_price() -> result::Result<u128, DispatchError> {
        let price: u128 = 1;
        Ok(price)
    }
}
