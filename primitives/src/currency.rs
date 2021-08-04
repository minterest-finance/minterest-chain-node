use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub enum OriginalAsset {
	MNT = 0,
	DOT = 1,
	KSM = 2,
	BTC = 3,
	ETH = 4,
}

impl OriginalAsset {
	pub fn as_wrap(self) -> Option<WrapToken> {
		match self {
			OriginalAsset::MNT => None,
			OriginalAsset::DOT => Some(WrapToken::DOT),
			OriginalAsset::KSM => Some(WrapToken::KSM),
			OriginalAsset::BTC => Some(WrapToken::BTC),
			OriginalAsset::ETH => Some(WrapToken::ETH),
		}
	}

	pub fn get_original_assets() -> &'static [OriginalAsset] {
		&[
			OriginalAsset::MNT,
			OriginalAsset::DOT,
			OriginalAsset::KSM,
			OriginalAsset::BTC,
			OriginalAsset::ETH,
		]
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub enum WrapToken {
	// Skip zero id
	DOT = 1,
	KSM = 2,
	BTC = 3,
	ETH = 4,
}

impl WrapToken {
	pub fn as_asset(self) -> OriginalAsset {
		match self {
			WrapToken::DOT => OriginalAsset::DOT,
			WrapToken::KSM => OriginalAsset::KSM,
			WrapToken::BTC => OriginalAsset::BTC,
			WrapToken::ETH => OriginalAsset::ETH,
		}
	}

	pub fn get_wrap_tokens() -> &'static [WrapToken] {
		&[WrapToken::DOT, WrapToken::KSM, WrapToken::BTC, WrapToken::ETH]
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, Hash)]
pub enum CurrencyId {
	Original(OriginalAsset),
	Wrap(WrapToken),
}

impl CurrencyId {
	pub fn is_native(&self) -> bool {
		*self == CurrencyId::Original(OriginalAsset::MNT)
	}
}

impl From<OriginalAsset> for CurrencyId {
	fn from(asset: OriginalAsset) -> Self {
		CurrencyId::Original(asset)
	}
}

impl From<WrapToken> for CurrencyId {
	fn from(token: WrapToken) -> Self {
		CurrencyId::Wrap(token)
	}
}
