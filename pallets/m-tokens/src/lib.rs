#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_event, decl_storage, decl_module,
};
use frame_system::{self as system};

// #[cfg(feature = "std")]
// use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub trait Trait: system::Trait {
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as system::Trait>::Event>;
}

decl_event! {
    pub enum Event {}
}

decl_storage! {
    trait Store for Module<T: Trait> as MTokens {

    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

    }
}
