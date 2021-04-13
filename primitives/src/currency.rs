use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::convert::TryFrom;
use sp_std::{prelude::Vec, vec};
#[allow(unused_imports)]
use CurrencyType::*;

macro_rules! create_currency_id {
	($(#[$meta:meta])*
	$vis:vis enum TokenSymbol {
        $($(#[$vmeta:meta])* $symbol:ident($name:expr, $deci:literal, $ctype:ident) = $val:literal,)*
    }) => {
        $(#[$meta])*
        $vis enum TokenSymbol {
            $($(#[$vmeta])* $symbol = $val,)*
        }

		impl TryFrom<u8> for TokenSymbol {
            type Error = ();

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $($val => Ok(TokenSymbol::$symbol),)*
                    _ => Err(()),
                }
            }
        }

		impl GetDecimals for CurrencyId {
			fn decimals(&self) -> u32 {
				match self {
					$(CurrencyId::Native(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::UnderlyingAsset(TokenSymbol::$symbol) => $deci,)*
					$(CurrencyId::WrappedToken(TokenSymbol::$symbol) => $deci,)*
				}
			}
		}

		impl CurrencyId {
			pub fn get_enabled_tokens_in_protocol(token_type: CurrencyType) -> Vec<CurrencyId> {
				let mut enabled_tokens = vec![];
				$(
					if token_type == $ctype {
						enabled_tokens.push(
							match $ctype {
								Native => CurrencyId::Native(TokenSymbol::$symbol),
								UnderlyingAsset => CurrencyId::UnderlyingAsset(TokenSymbol::$symbol),
								WrappedToken => CurrencyId::WrappedToken(TokenSymbol::$symbol),
							}
						);
					}
				)*
				enabled_tokens
			}
		}

		$(pub const $symbol: CurrencyId = match $ctype {
			Native => CurrencyId::Native(TokenSymbol::$symbol),
			UnderlyingAsset => CurrencyId::UnderlyingAsset(TokenSymbol::$symbol),
			WrappedToken => CurrencyId::WrappedToken(TokenSymbol::$symbol),
		};)*
	}
}

create_currency_id! {
	// Convention: the wrapped token follows immediately after the underlying token.
	// Wrapped token ID = Underlying Asset ID + 1;
	// Underlying Asset ID  = Wrapped token ID - 1;
	# [derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
	# [cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	# [repr(u8)]
	pub enum TokenSymbol {
		MNT("Minterest", 18, Native) = 0,
		DOT("Polkadot", 10, UnderlyingAsset) = 1,
		MDOT("Polkadot", 10, WrappedToken) = 2,
		KSM("Kusama", 12, UnderlyingAsset) = 3,
		MKSM("Kusama", 12, WrappedToken) = 4,
		BTC("Bitcoin", 8, UnderlyingAsset) = 5,
		MBTC("Bitcoin", 8, WrappedToken) = 6,
		ETH("Ethereum", 18, UnderlyingAsset) = 7,
		METH("Ethereum", 18, WrappedToken) = 8,
	}
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	Native(TokenSymbol),
	UnderlyingAsset(TokenSymbol),
	WrappedToken(TokenSymbol),
}

impl CurrencyId {
	pub fn is_native_currency_id(&self) -> bool {
		matches!(self, CurrencyId::Native(_))
	}

	pub fn is_enabled_underlying_asset_id(&self) -> bool {
		matches!(self, CurrencyId::UnderlyingAsset(_))
	}

	pub fn is_enabled_wrapped_token_id(&self) -> bool {
		matches!(self, CurrencyId::WrappedToken(_))
	}

	pub fn underlying_asset_id(&self) -> Option<CurrencyId> {
		if self.is_enabled_wrapped_token_id() {
			match self {
				CurrencyId::WrappedToken(currency_id) => Some(CurrencyId::UnderlyingAsset(
					TokenSymbol::try_from(*currency_id as u8 - 1_u8).ok()?,
				)),
				_ => None,
			}
		} else {
			None
		}
	}

	pub fn wrapped_token_id(&self) -> Option<CurrencyId> {
		if self.is_enabled_underlying_asset_id() {
			match self {
				CurrencyId::UnderlyingAsset(currency_id) => Some(CurrencyId::WrappedToken(
					TokenSymbol::try_from(*currency_id as u8 + 1_u8).ok()?,
				)),
				_ => None,
			}
		} else {
			None
		}
	}
}

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum CurrencyType {
	Native,
	UnderlyingAsset,
	WrappedToken,
}

pub trait GetDecimals {
	fn decimals(&self) -> u32;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn currency_type_identification_should_work() {
		assert!(MNT.is_native_currency_id());
		assert!(!DOT.is_native_currency_id());
		assert!(DOT.is_enabled_underlying_asset_id());
		assert!(!MDOT.is_enabled_underlying_asset_id());
		assert!(!ETH.is_enabled_wrapped_token_id());
		assert!(METH.is_enabled_wrapped_token_id());
	}

	#[test]
	fn get_decimal_should_work() {
		assert_eq!(MNT.decimals(), 18);
		assert_eq!(DOT.decimals(), 10);
	}

	#[test]
	fn wrapped_token_id_should_work() {
		assert_eq!(MNT.wrapped_token_id(), None);
		assert_eq!(DOT.wrapped_token_id(), Some(MDOT));
		assert_eq!(METH.wrapped_token_id(), None);
	}

	#[test]
	fn get_underlying_asset_id_by_wrapped_id_should_work() {
		assert_eq!(MNT.underlying_asset_id(), None);
		assert_eq!(MDOT.underlying_asset_id(), Some(DOT));
		assert_eq!(ETH.underlying_asset_id(), None);
	}

	#[test]
	fn get_enabled_tokens_in_protocol_should_work() {
		assert_eq!(CurrencyId::get_enabled_tokens_in_protocol(Native), vec![MNT]);
		assert_eq!(
			CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset),
			vec![DOT, KSM, BTC, ETH]
		);
		assert_eq!(
			CurrencyId::get_enabled_tokens_in_protocol(WrappedToken),
			vec![MDOT, MKSM, MBTC, METH]
		);
	}
}
