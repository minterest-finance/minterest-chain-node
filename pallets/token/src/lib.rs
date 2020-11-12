#![cfg_attr(not(feature = "std"), no_std)]
/// Pallet implementing the ERC20 token factory API
/// You can use mint to create tokens or burn created tokens
/// and transfer tokens on substrate side freely or operate with total_supply

use frame_support::{
    codec::{Decode, Encode},
    decl_storage, decl_event, decl_module,
};
use frame_system::{self as system};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

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

    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as balances::Trait>::Balance,
    {
        Transfer(AccountId, AccountId, Balance),
        Approval(AccountId, AccountId, Balance),
        Mint(AccountId, Balance),
        Burn(AccountId, Balance),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;
    }
}
