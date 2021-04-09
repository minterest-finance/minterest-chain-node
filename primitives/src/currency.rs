use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::convert::TryFrom;
#[allow(unused_imports)]
use CurrencyType::*;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	MNT = 0,
	DOT,
	KSM,
	BTC,
	ETH,
	MDOT,
	MKSM,
	MBTC,
	METH,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct CurrencyPair {
	pub underlying_id: CurrencyId,
	pub wrapped_id: CurrencyId,
}

impl CurrencyPair {
	pub fn new(underlying_id: CurrencyId, wrapped_id: CurrencyId) -> Self {
		Self {
			underlying_id,
			wrapped_id,
		}
	}
}

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

		impl GetDecimals for NEWCurrencyId {
			fn decimals(&self) -> u32 {
				match self {
					$(NEWCurrencyId::Native(TokenSymbol::$symbol) => $deci,)*
					$(NEWCurrencyId::UnderlyingAsset(TokenSymbol::$symbol) => $deci,)*
					$(NEWCurrencyId::WrappedToken(TokenSymbol::$symbol) => $deci,)*
				}
			}
		}

		$(pub const $symbol: NEWCurrencyId = match $ctype {
			Native => NEWCurrencyId::Native(TokenSymbol::$symbol),
			UnderlyingAsset => NEWCurrencyId::UnderlyingAsset(TokenSymbol::$symbol),
			WrappedToken => NEWCurrencyId::WrappedToken(TokenSymbol::$symbol),
		};)*

		// {let mut EnabledUnderlyingAssetsIds = Vec::new();
		// $(EnabledUnderlyingAssetsIds.push($symbol);)*
		// 	EnabledUnderlyingAssetsIds
		// }

		 // let EnabledUnderlyingAssetsIds: Vec<NEWCurrencyId> = vec![$(NEWCurrencyId::Native(TokenSymbol::$symbol),)*];

		// pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
		// 	CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
		// 	CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
		// 	CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
		// 	CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
		// ];
	}
}

create_currency_id! {
	// Convention: the wrapped token follows immediately after the underlying token.
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
pub enum NEWCurrencyId {
	Native(TokenSymbol),
	UnderlyingAsset(TokenSymbol),
	WrappedToken(TokenSymbol),
}

impl NEWCurrencyId {
	pub fn is_native_currency_id(&self) -> bool {
		matches!(self, NEWCurrencyId::Native(_))
	}
	pub fn is_underlying_asset_id(&self) -> bool {
		matches!(self, NEWCurrencyId::UnderlyingAsset(_))
	}
	pub fn is_wrapped_token_id(&self) -> bool {
		matches!(self, NEWCurrencyId::WrappedToken(_))
	}
	pub fn get_underlying_asset_id_by_wrapped_id(&self) -> Option<NEWCurrencyId> {
		if self.is_wrapped_token_id() {
			match self {
				NEWCurrencyId::UnderlyingAsset(currency_id) => Some(NEWCurrencyId::WrappedToken(
					TokenSymbol::try_from(*currency_id as u8 - 1_u8).ok()?,
				)),
				_ => None,
			}
		} else {
			None
		}
	}
	pub fn get_wrapped_id_by_underlying_asset_id(&self) -> Option<NEWCurrencyId> {
		if self.is_underlying_asset_id() {
			match self {
				NEWCurrencyId::UnderlyingAsset(currency_id) => Some(NEWCurrencyId::WrappedToken(
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
		assert!(DOT.is_underlying_asset_id());
		assert!(!MDOT.is_underlying_asset_id());
		assert!(!ETH.is_wrapped_token_id());
		assert!(METH.is_wrapped_token_id());
	}

	#[test]
	fn get_decimal_should_work() {
		assert_eq!(MNT.decimals(), 18);
		assert_eq!(DOT.decimals(), 10);
	}

	#[test]
	fn get_wrapped_id_by_underlying_asset_id_should_work() {
		assert_eq!(MNT.get_wrapped_id_by_underlying_asset_id(), None);
		assert_eq!(DOT.get_wrapped_id_by_underlying_asset_id(), Some(MDOT));
		assert_eq!(METH.get_wrapped_id_by_underlying_asset_id(), None);
	}

	#[test]
	fn get_underlying_asset_id_by_wrapped_id_should_work() {
		assert_eq!(MNT.get_underlying_asset_id_by_wrapped_id(), None);
		assert_eq!(MDOT.get_underlying_asset_id_by_wrapped_id(), Some(DOT));
		assert_eq!(ETH.get_underlying_asset_id_by_wrapped_id(), None);
	}
}
