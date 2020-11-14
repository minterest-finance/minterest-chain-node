#![cfg_attr(not(feature = "std"), no_std)]
/// Pallet implementing the ERC20 token factory API
/// You can use mint to create tokens or burn created tokens
/// and transfer tokens on substrate side freely or operate with total_supply

use frame_support::{
    codec::{Decode, Encode},
    decl_storage, decl_event, decl_module, ensure,
    dispatch::DispatchResult,
};
use frame_system::{self as system, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::traits::{Zero};
use sp_std::prelude::Vec;
use num_traits::ops::checked::{CheckedAdd};

#[cfg(test)]
mod tests;

type Result<T> = core::result::Result<T, &'static str>;

//token factory
pub type TokenId = u32;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Deserialize, Serialize, Debug))]
pub struct Token {
    pub id: TokenId,
    pub decimals: u16,
    pub symbol: Vec<u8>,
}

pub trait Trait: balances::Trait + system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as TokenStorage {
        pub TotalSupply get(fn total_supply): map hasher(opaque_blake2_256) TokenId => T::Balance;
        pub Balance get(fn balance_of): map hasher(opaque_blake2_256) (TokenId, T::AccountId) => T::Balance;
        pub Allowance get(fn allowance_of): map hasher(opaque_blake2_256) (TokenId, T::AccountId, T::AccountId) => T::Balance;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as balances::Trait>::Balance,
    {
        // TODO #1 Add comments to display on the frontend
        Transfer(AccountId, AccountId, Balance),
        Approval(AccountId, AccountId, Balance),
        Mint(AccountId, Balance),
        Burn(AccountId, Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 10_000]
        fn mint(
            origin,
            to: T::AccountId,
            token_id: TokenId,
            #[compact] amount: T::Balance
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;



            Ok(())
        }


    }
}

impl<T: Trait> Module<T> {
    pub fn _burn(_token_id: TokenId, _from: T::AccountId, _amount: T::Balance) -> Result<()> {
        Ok(())
    }

    pub fn _mint(token_id: TokenId, to: T::AccountId, amount: T::Balance) -> Result<()> {
        ensure!(!amount.is_zero(), "Amount should be non-zero");

        let old_balance = <Balance<T>>::get((token_id, to.clone()));
        let next_balance = old_balance
            .checked_add(&amount)
            .ok_or("Overflow adding to balance")?;
        let next_total = Self::total_supply(0)
            .checked_add(&amount)
            .ok_or("Overflow adding to total supply")?;

        <Balance<T>>::insert((token_id, to.clone()), next_balance);
        <TotalSupply<T>>::insert(token_id, next_total);

        Ok(())
    }
}
