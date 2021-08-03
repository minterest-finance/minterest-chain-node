use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::ops::Deref;

pub const MNT: OriginalAsset = OriginalAsset(TokenSymbol::MNT);
pub const DOT: OriginalAsset = OriginalAsset(TokenSymbol::DOT);
pub const KSM: OriginalAsset = OriginalAsset(TokenSymbol::KSM);
pub const BTC: OriginalAsset = OriginalAsset(TokenSymbol::BTC);
pub const ETH: OriginalAsset = OriginalAsset(TokenSymbol::ETH);

# [derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
# [cfg_attr(feature = "std", derive(Serialize, Deserialize))]
# [repr(u8)]
pub enum TokenSymbol {
	MNT,
	DOT,
	KSM,
	BTC,
	ETH,
}

impl TokenSymbol {
	pub fn decimals(&self) -> u32 {
		match self {
			TokenSymbol::MNT => 18,
			TokenSymbol::DOT => 10,
			TokenSymbol::KSM => 12,
			TokenSymbol::BTC => 8,
			TokenSymbol::ETH => 18,
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub struct OriginalAsset(TokenSymbol);

impl Deref for OriginalAsset {
    type Target = TokenSymbol;
    fn deref(&self) -> &TokenSymbol { &self.0 }
}

impl OriginalAsset {
	pub fn as_wrap(self) -> Option<WrapToken> {
		match self.0 {
			TokenSymbol::MNT => None,
			symbol @ _ => Some(WrapToken(symbol)),
		}
	}

	pub fn as_currency(self) -> CurrencyId {
		match self {
			OriginalAsset(TokenSymbol::MNT) => CurrencyId::Native,
			asset @ _ => CurrencyId::OriginalAsset(asset),
		}
	}

	pub fn get_original_assets() -> &'static [OriginalAsset] {
		&[
			OriginalAsset(TokenSymbol::MNT),
			OriginalAsset(TokenSymbol::DOT),
			OriginalAsset(TokenSymbol::KSM),
			OriginalAsset(TokenSymbol::BTC),
			OriginalAsset(TokenSymbol::ETH),
		]
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub struct WrapToken(TokenSymbol);

impl Deref for WrapToken {
    type Target = TokenSymbol;
    fn deref(&self) -> &TokenSymbol { &self.0 }
}

impl WrapToken {
	pub fn as_asset(self) -> OriginalAsset {
		OriginalAsset(self.0)
	}

	pub fn as_currency(self) -> CurrencyId {
		CurrencyId::WrapToken(self)
	}

	pub fn is_valid(self) -> bool {
		self.0 != TokenSymbol::MNT
	}

	pub fn get_wrap_tokens() -> &'static [WrapToken] {
		&[
			WrapToken(TokenSymbol::DOT),
			WrapToken(TokenSymbol::KSM),
			WrapToken(TokenSymbol::BTC),
			WrapToken(TokenSymbol::ETH),
		]
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub enum CurrencyId {
	Native,
	OriginalAsset(OriginalAsset),
	WrapToken(WrapToken),
}

impl Deref for CurrencyId {
    type Target = TokenSymbol;
    fn deref(&self) -> &TokenSymbol {
		match self {
			CurrencyId::Native => &TokenSymbol::MNT,
			CurrencyId::OriginalAsset(tk) => tk,
			CurrencyId::WrapToken(tk) => tk,
		}
	}
}
