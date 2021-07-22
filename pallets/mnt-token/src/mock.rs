#![cfg(test)]

use crate as mnt_token;
use frame_support::{construct_runtime, ord_parameter_types, pallet_prelude::*, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use liquidity_pools::{PoolData, PoolUserData};
pub use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_runtime::{
	testing::{Header, H256},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One, Zero},
	FixedPointNumber,
};
pub use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		MntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		MinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
		Controller: controller::{Pallet, Storage, Call, Event, Config<T>},
	}
);

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"min/lqdy");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub const MntTokenPalletId: PalletId = PalletId(*b"min/mntt");
	pub MntTokenAccountId: AccountId = MntTokenPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

mock_impl_system_config!(Runtime);
mock_impl_orml_tokens_config!(Runtime);
mock_impl_orml_currencies_config!(Runtime);
mock_impl_liquidity_pools_config!(Runtime);
mock_impl_minterest_model_config!(Runtime, ZeroAdmin);
mock_impl_controller_config!(Runtime, ZeroAdmin);
mock_impl_balances_config!(Runtime);
mock_impl_mnt_token_config!(Runtime, ZeroAdmin);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
	pub const BlockHashCount: u64 = 250;
}

pub struct MockPriceSource;

impl PricesManager<CurrencyId> for MockPriceSource {
	fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
		match currency_id {
			DOT => return Some(Price::saturating_from_rational(5, 10)), // 0.5 USD
			ETH => return Some(Price::saturating_from_rational(15, 10)), // 1.5 USD
			KSM => return Some(Price::saturating_from_integer(2)),      // 2 USD
			BTC => return Some(Price::saturating_from_integer(3)),      // 3 USD
			_ => return None,
		}
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, PoolData)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	minted_pools: Vec<(CurrencyId, Balance)>,
	mnt_claim_threshold: Balance,
}

pub const BOB: AccountId = 2;

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			pools: vec![],
			minted_pools: vec![],
			pool_user_data: vec![],
			endowed_accounts: vec![],
			mnt_claim_threshold: Balance::zero(),
		}
	}
}

impl ExtBuilder {
	pub fn mnt_enabled_pools(mut self, pools: Vec<(CurrencyId, Balance)>) -> Self {
		self.minted_pools = pools;
		self
	}

	pub fn enable_minting_for_all_pools(mut self, speed: Balance) -> Self {
		self.minted_pools = vec![(KSM, speed), (DOT, speed), (ETH, speed), (BTC, speed)];
		self
	}

	pub fn set_mnt_claim_threshold(mut self, threshold: u128) -> Self {
		self.mnt_claim_threshold = threshold * DOLLARS;
		self
	}

	pub fn pool_borrow_underlying(mut self, pool_id: CurrencyId, pool_borrowed: Balance) -> Self {
		self.pools.push((
			pool_id,
			PoolData {
				borrowed: pool_borrowed,
				borrow_index: Rate::saturating_from_rational(15, 10),
				protocol_interest: Balance::zero(),
			},
		));
		self
	}

	pub fn mnt_account_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts.push((MntToken::get_account_id(), MNT, balance));
		self
	}

	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	pub fn pool_user_data(
		mut self,
		pool_id: CurrencyId,
		user: AccountId,
		borrowed: Balance,
		interest_index: Rate,
		is_collateral: bool,
	) -> Self {
		self.pool_user_data.push((
			pool_id,
			user,
			PoolUserData {
				borrowed,
				interest_index,
				is_collateral,
			},
		));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		liquidity_pools::GenesisConfig::<Runtime> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == MNT)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != MNT)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mnt_token::GenesisConfig::<Runtime> {
			mnt_claim_threshold: self.mnt_claim_threshold,
			minted_pools: self.minted_pools,
			_phantom: PhantomData,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
