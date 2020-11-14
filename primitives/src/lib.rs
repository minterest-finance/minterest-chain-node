#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{
    RuntimeDebug,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Signed version of Balance
pub type Amount = i128;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    MINT = 0,
    METH,
    MDAI,
}
