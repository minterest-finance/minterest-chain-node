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
        $($symbol:ident($name:expr, $deci:literal, $ctype:ident) = $val:literal,)*
    }) => {
        $(#[$meta])*
        $vis enum TokenSymbol {
            $($symbol = $val,)*
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
	# [derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
	# [cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	# [repr(u8)]
	pub enum TokenSymbol {
		MNT("Minterest", 18, Native) = 0,
		MMNT("Minterest", 18, WrappedToken) = 1,
		DOT("Polkadot", 10, UnderlyingAsset) = 2,
		MDOT("Polkadot", 10, WrappedToken) = 3,
		KSM("Kusama", 12, UnderlyingAsset) = 4,
		MKSM("Kusama", 12, WrappedToken) = 5,
		BTC("Bitcoin", 8, UnderlyingAsset) = 6,
		MBTC("Bitcoin", 8, WrappedToken) = 7,
		ETH("Ethereum", 18, UnderlyingAsset) = 8,
		METH("Ethereum", 18, WrappedToken) = 9,

	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub enum CurrencyId {
	Native(TokenSymbol),
	UnderlyingAsset(TokenSymbol),
	WrappedToken(TokenSymbol),
}

impl CurrencyId {
	pub fn is_native_currency_id(&self) -> bool {
		matches!(self, CurrencyId::Native(_))
	}

	pub fn is_supported_underlying_asset(&self) -> bool {
		matches!(self, CurrencyId::UnderlyingAsset(_))
	}

	pub fn is_supported_wrapped_asset(&self) -> bool {
		matches!(self, CurrencyId::WrappedToken(_))
	}

	pub fn underlying_asset(&self) -> Option<CurrencyId> {
		match (self.is_supported_wrapped_asset(), self) {
			(true, CurrencyId::WrappedToken(currency_id)) => Some(CurrencyId::UnderlyingAsset(
				TokenSymbol::try_from(*currency_id as u8 - 1_u8).ok()?,
			)),
			_ => None,
		}
	}

	pub fn wrapped_asset(&self) -> Option<CurrencyId> {
		match (
			self.is_supported_underlying_asset() || self.is_native_currency_id(),
			self,
		) {
			(true, CurrencyId::UnderlyingAsset(currency_id)) => Some(CurrencyId::WrappedToken(
				TokenSymbol::try_from(*currency_id as u8 + 1_u8).ok()?,
			)),
			(true, CurrencyId::Native(currency_id)) => Some(CurrencyId::WrappedToken(
				TokenSymbol::try_from(*currency_id as u8 + 1_u8).ok()?,
			)),
			_ => None,
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
		assert!(DOT.is_supported_underlying_asset());
		assert!(!MDOT.is_supported_underlying_asset());
		assert!(!ETH.is_supported_wrapped_asset());
		assert!(METH.is_supported_wrapped_asset());
		assert!(!MMNT.is_native_currency_id());
		assert!(!MMNT.is_supported_underlying_asset());
		assert!(MMNT.is_supported_wrapped_asset());
	}

	#[test]
	fn get_decimal_should_work() {
		assert_eq!(MNT.decimals(), 18);
		assert_eq!(DOT.decimals(), 10);
	}

	#[test]
	fn wrapped_asset_should_work() {
		assert_eq!(MNT.wrapped_asset(), Some(MMNT));
		assert_eq!(DOT.wrapped_asset(), Some(MDOT));
		assert_eq!(METH.wrapped_asset(), None);
	}

	#[test]
	fn underlying_asset_should_work() {
		assert_eq!(MNT.underlying_asset(), None);
		assert_eq!(MDOT.underlying_asset(), Some(DOT));
		assert_eq!(ETH.underlying_asset(), None);
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
			vec![MMNT, MDOT, MKSM, MBTC, METH]
		);
	}
}
