//! Mocks for the minterest-protocol module.

use super::*;
use crate as minterest_protocol;
use controller::{ControllerData, PauseKeeper};
use frame_support::{ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
use minterest_model::MinterestModelData;
pub use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::{PricesManager, RiskManager};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One},
	FixedPointNumber,
};
pub use test_helper::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Controller: controller::{Pallet, Storage, Call, Event, Config<T>},
		TestMinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
		TestMinterestProtocol: minterest_protocol::{Pallet, Storage, Call, Event<T>},
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestLiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestDex: dex::{Pallet, Storage, Call, Event<T>},
		TestMntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestWhitelist: whitelist_module::{Pallet, Storage, Call, Event<T>, Config<T>},
	}
);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
	pub const OneAlice: AccountId = 1;
}

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"min/lqdy");
	pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"min/lqdn");
	pub const MntTokenPalletId: PalletId = PalletId(*b"min/mntt");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
	pub MntTokenAccountId: AccountId = MntTokenPalletId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

mock_impl_system_config!(Test);
mock_impl_orml_tokens_config!(Test);
mock_impl_orml_currencies_config!(Test);
mock_impl_liquidity_pools_config!(Test);
mock_impl_liquidation_pools_config!(Test);
mock_impl_controller_config!(Test, OneAlice);
mock_impl_minterest_model_config!(Test, OneAlice);
mock_impl_dex_config!(Test);
mock_impl_minterest_protocol_config!(Test, OneAlice);
mock_impl_mnt_token_config!(Test, OneAlice);
mock_impl_balances_config!(Test);
mock_impl_whitelist_module_config!(Test, OneAlice);

pub struct TestRiskManager;

impl RiskManager for TestRiskManager {
	fn create_pool(
		_currency_id: CurrencyId,
		_max_attempts: u8,
		_min_partial_liquidation_sum: u128,
		_threshold: Rate,
		_liquidation_fee: Rate,
	) -> DispatchResult {
		Ok(())
	}
}

pub struct MockPriceSource;

impl PricesManager<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	controller_data: Vec<(CurrencyId, ControllerData<BlockNumber>)>,
	minterest_model_params: Vec<(CurrencyId, MinterestModelData)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				// seed: initial DOTs
				(ALICE, DOT, ONE_HUNDRED),
				(ALICE, ETH, ONE_HUNDRED),
				(ALICE, KSM, ONE_HUNDRED),
				(BOB, DOT, ONE_HUNDRED),
				// seed: initial interest, equal 10_000$
				(TestPools::pools_account_id(), ETH, TEN_THOUSAND),
				(TestPools::pools_account_id(), DOT, TEN_THOUSAND),
				// seed: initial interest = 10_000$, initial pool balance = 1_000_000$
				(TestPools::pools_account_id(), KSM, ONE_MILL),
				// seed: initial MNT treasury = 1_000_000$
				(TestMntToken::get_account_id(), MNT, ONE_MILL),
			],
			pools: vec![],
			controller_data: vec![
				(
					ETH,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					DOT,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					KSM,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
				(
					BTC,
					ControllerData {
						last_interest_accrued_block: 0,
						protocol_interest_factor: Rate::saturating_from_rational(1, 10), // 10%
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),        // 0.5%
						collateral_factor: Rate::saturating_from_rational(9, 10),        // 90%
						borrow_cap: None,
						protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
					},
				),
			],
			minterest_model_params: vec![],
		}
	}
}
impl ExtBuilder {
	pub fn set_controller_data(mut self, pools: Vec<(CurrencyId, ControllerData<BlockNumber>)>) -> Self {
		self.controller_data = pools;
		self
	}

	pub fn set_minterest_model_params(mut self, pools: Vec<(CurrencyId, MinterestModelData)>) -> Self {
		self.minterest_model_params = pools;
		self
	}

	pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	pub fn pool_with_params(
		mut self,
		pool_id: CurrencyId,
		total_borrowed: Balance,
		borrow_index: Rate,
		total_protocol_interest: Balance,
	) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				total_borrowed,
				borrow_index,
				total_protocol_interest,
			},
		));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		pallet_balances::GenesisConfig::<Test> {
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

		orml_tokens::GenesisConfig::<Test> {
			balances: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != MNT)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.pools,
			pool_user_data: vec![
				(
					DOT,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						is_collateral: true,
						liquidation_attempts: 3,
					},
				),
				(
					ETH,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						is_collateral: false,
						liquidation_attempts: 0,
					},
				),
				(
					KSM,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						is_collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					BTC,
					ALICE,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						is_collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					DOT,
					BOB,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						is_collateral: true,
						liquidation_attempts: 0,
					},
				),
				(
					BTC,
					BOB,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::from_inner(0),
						is_collateral: true,
						liquidation_attempts: 0,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<Test> {
			controller_dates: self.controller_data,
			pause_keepers: vec![
				(ETH, PauseKeeper::all_unpaused()),
				(DOT, PauseKeeper::all_unpaused()),
				(KSM, PauseKeeper::all_paused()),
				(BTC, PauseKeeper::all_unpaused()),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		minterest_model::GenesisConfig::<Test> {
			minterest_model_params: self.minterest_model_params,
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mnt_token::GenesisConfig::<Test> {
			mnt_claim_threshold: 100 * DOLLARS,
			minted_pools: vec![(DOT, DOLLARS / 10), (ETH, DOLLARS / 10)],
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn create_dummy_pool_init_data() -> PoolInitData {
	PoolInitData {
		kink: Rate::saturating_from_rational(2, 3),
		base_rate_per_block: Rate::saturating_from_rational(1, 3),
		multiplier_per_block: Rate::saturating_from_rational(2, 4),
		jump_multiplier_per_block: Rate::saturating_from_rational(1, 2),
		protocol_interest_factor: Rate::saturating_from_rational(1, 10),
		max_borrow_rate: Rate::saturating_from_rational(5, 1000),
		collateral_factor: Rate::saturating_from_rational(9, 10),
		protocol_interest_threshold: 100000,
		deviation_threshold: Rate::saturating_from_rational(5, 100),
		balance_ratio: Rate::saturating_from_rational(2, 10),
		max_attempts: 3,
		min_partial_liquidation_sum: 100 * DOLLARS,
		threshold: Rate::saturating_from_rational(103, 100),
		liquidation_fee: Rate::saturating_from_rational(105, 100),
	}
}
