//! Mocks for the minterest-protocol module.

use super::*;
use crate as minterest_protocol;
use controller::{ControllerData, PauseKeeper};
use frame_support::{assert_ok, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
pub use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
use minterest_primitives::{Balance, CurrencyId, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PriceProvider;
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	FixedPointNumber,
};
use sp_std::cell::RefCell;
pub use test_helper::*;

pub type AccountId = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		Controller: controller::{Module, Storage, Call, Event, Config<T>},
		MinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		TestProtocol: minterest_protocol::{Module, Storage, Call, Event<T>},
		TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
		TestLiquidationPools: liquidation_pools::{Module, Storage, Call, Event<T>, Config<T>},
		TestDex: dex::{Module, Storage, Call, Event<T>},
		TestMntToken: mnt_token::{Module, Storage, Call, Event<T>, Config<T>},
	}
);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
	pub const OneAlice: AccountId = 1;
}

parameter_types! {
	pub const LiquidityPoolsModuleId: PalletId = PalletId(*b"min/lqdy");
	pub const LiquidationPoolsModuleId: PalletId = PalletId(*b"min/lqdn");
	pub const MntTokenModuleId: PalletId = PalletId(*b"min/mntt");
	pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
	pub LiquidationPoolAccountId: AccountId = LiquidationPoolsModuleId::get().into_account();
	pub MntTokenAccountId: AccountId = MntTokenModuleId::get().into_account();
	pub InitialExchangeRate: Rate = Rate::one();
	pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
	pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
}

pub struct WhitelistMembers;
mock_impl_system_config!(Test);
mock_impl_orml_tokens_config!(Test);
mock_impl_orml_currencies_config!(Test);
mock_impl_liquidity_pools_config!(Test);
mock_impl_liquidation_pools_config!(Test);
mock_impl_controller_config!(Test, OneAlice);
mock_impl_minterest_model_config!(Test, OneAlice);
mock_impl_dex_config!(Test);
mock_impl_minterest_protocol_config!(Test);
mock_impl_mnt_token_config!(Test, OneAlice);
mock_impl_balances_config!(Test);

pub struct MockPriceSource;

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

thread_local! {
	static TWO: RefCell<Vec<u64>> = RefCell::new(vec![2]);
}

impl Contains<u64> for WhitelistMembers {
	fn contains(who: &AccountId) -> bool {
		TWO.with(|v| v.borrow().contains(who))
	}

	fn sorted_members() -> Vec<u64> {
		TWO.with(|v| v.borrow().clone())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(new: &u128) {
		TWO.with(|v| {
			let mut members = v.borrow_mut();
			members.push(*new);
			members.sort();
		})
	}
}

pub const ALICE: AccountId = 1;
pub fn alice() -> Origin {
	Origin::signed(ALICE)
}
pub const BOB: AccountId = 2;
pub fn bob() -> Origin {
	Origin::signed(BOB)
}
pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub const ONE_MILL_DOLLARS: Balance = 1_000_000 * DOLLARS;
pub const ONE_HUNDRED_DOLLARS: Balance = 100 * DOLLARS;
pub const TEN_THOUSAND_DOLLARS: Balance = 10_000 * DOLLARS;
pub const PROTOCOL_INTEREST_TRANSFER_THRESHOLD: Balance = 1_000_000_000_000_000_000_000;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				// seed: initial DOTs
				(ALICE, DOT, ONE_HUNDRED_DOLLARS),
				(ALICE, ETH, ONE_HUNDRED_DOLLARS),
				(ALICE, KSM, ONE_HUNDRED_DOLLARS),
				(BOB, DOT, ONE_HUNDRED_DOLLARS),
				// seed: initial interest, equal 10_000$
				(TestPools::pools_account_id(), ETH, TEN_THOUSAND_DOLLARS),
				(TestPools::pools_account_id(), DOT, TEN_THOUSAND_DOLLARS),
				// seed: initial interest = 10_000$, initial pool balance = 1_000_000$
				(TestPools::pools_account_id(), KSM, ONE_MILL_DOLLARS),
				// seed: initial MNT treasury = 1_000_000$
				(TestMntToken::get_account_id(), MNT, ONE_MILL_DOLLARS),
			],
			pools: vec![],
		}
	}
}
impl ExtBuilder {
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
			endowed_accounts: self
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
						liquidation_attempts: 0,
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
			controller_dates: vec![
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
			pause_keepers: vec![
				(
					ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					KSM,
					PauseKeeper {
						deposit_paused: true,
						redeem_paused: true,
						borrow_paused: true,
						repay_paused: true,
						transfer_paused: true,
					},
				),
				(
					BTC,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
			],
			whitelist_mode: false,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mnt_token::GenesisConfig::<Test> {
			mnt_rate: 100_000_000_000_000_000, // 0.1
			mnt_claim_threshold: 100 * DOLLARS,
			minted_pools: vec![DOT, ETH],
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn set_block_number_and_refresh_speeds(n: u64) {
	System::set_block_number(n);
	assert_ok!(TestMntToken::refresh_mnt_speeds());
}
