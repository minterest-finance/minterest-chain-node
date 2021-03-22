//! # Integration Tests Module
//!
//! ## Overview
//!
//! TODO: add overview.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types};
	use frame_system::{self as system, EnsureSignedBy};
	use liquidity_pools::{Pool, PoolUserData};
	use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Price, Rate};
	use orml_currencies::Currency;
	use orml_traits::parameter_type_with_key;
	use orml_traits::MultiCurrency;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, Zero},
		FixedPointNumber, ModuleId,
	};

	use controller::{ControllerData, PauseKeeper};
	use frame_support::traits::Contains;
	use minterest_model::MinterestModelData;
	use minterest_protocol::Error as MinterestProtocolError;
	use pallet_traits::{PoolsManager, PriceProvider};
	use sp_std::cell::RefCell;

	mod controller_tests;
	mod liquidity_pools_tests;
	mod minterest_model_tests;
	mod minterest_protocol_tests;
	mod scenario_tests;

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
			Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
			Currencies: orml_currencies::{Module, Call, Event<T>},
			MTokens: m_tokens::{Module, Storage, Call, Event<T>},
			MinterestProtocol: minterest_protocol::{Module, Storage, Call, Event<T>},
			TestPools: liquidity_pools::{Module, Storage, Call, Config<T>},
			TestController: controller::{Module, Storage, Call, Event, Config<T>},
			MinterestModel: minterest_model::{Module, Storage, Call, Event, Config},
		}
	);

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const SS58Prefix: u8 = 42;
	}

	pub type AccountId = u64;

	impl system::Config for Test {
		type BaseCallFilter = ();
		type BlockWeights = ();
		type BlockLength = ();
		type DbWeight = ();
		type Origin = Origin;
		type Call = Call;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = SS58Prefix;
	}

	type Amount = i128;

	parameter_type_with_key! {
		pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
			Default::default()
		};
	}

	impl orml_tokens::Config for Test {
		type Event = Event;
		type Balance = Balance;
		type Amount = Amount;
		type CurrencyId = CurrencyId;
		type WeightInfo = ();
		type ExistentialDeposits = ExistentialDeposits;
		type OnDust = ();
	}

	parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MNT;
	}

	type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

	impl orml_currencies::Config for Test {
		type Event = Event;
		type MultiCurrency = orml_tokens::Module<Test>;
		type NativeCurrency = NativeCurrency;
		type GetNativeCurrencyId = GetNativeCurrencyId;
		type WeightInfo = ();
	}

	impl m_tokens::Config for Test {
		type Event = Event;
		type MultiCurrency = orml_tokens::Module<Test>;
	}

	parameter_types! {
		pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/lqdy");
		pub LiquidityPoolAccountId: AccountId = LiquidityPoolsModuleId::get().into_account();
		pub InitialExchangeRate: Rate = Rate::one();
		pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
			CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
			CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
			CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
			CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
		];
		pub EnabledUnderlyingAssetId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
				.map(|currency_pair| currency_pair.underlying_id)
				.collect();
		pub EnabledWrappedTokensId: Vec<CurrencyId> = EnabledCurrencyPair::get().iter()
				.map(|currency_pair| currency_pair.wrapped_id)
				.collect();
	}

	pub struct MockPriceSource;

	impl PriceProvider<CurrencyId> for MockPriceSource {
		fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
			Some(Price::one())
		}

		fn lock_price(_currency_id: CurrencyId) {}

		fn unlock_price(_currency_id: CurrencyId) {}
	}

	impl liquidity_pools::Config for Test {
		type MultiCurrency = orml_tokens::Module<Test>;
		type PriceSource = MockPriceSource;
		type ModuleId = LiquidityPoolsModuleId;
		type LiquidityPoolAccountId = LiquidityPoolAccountId;
		type InitialExchangeRate = InitialExchangeRate;
		type EnabledCurrencyPair = EnabledCurrencyPair;
		type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
		type EnabledWrappedTokensId = EnabledWrappedTokensId;
	}

	thread_local! {
		static FOUR: RefCell<Vec<u64>> = RefCell::new(vec![4]);
	}

	pub struct Four;
	impl Contains<u64> for Four {
		fn sorted_members() -> Vec<u64> {
			FOUR.with(|v| v.borrow().clone())
		}
		#[cfg(feature = "runtime-benchmarks")]
		fn add(new: &u128) {
			TEN_TO_FOURTEEN.with(|v| {
				let mut members = v.borrow_mut();
				members.push(*new);
				members.sort();
			})
		}
	}

	impl minterest_protocol::Config for Test {
		type Event = Event;
		type Borrowing = liquidity_pools::Module<Test>;
		type ManagerLiquidityPools = liquidity_pools::Module<Test>;
		type WhitelistMembers = Four;
	}

	parameter_types! {
		pub const MaxBorrowCap: Balance = MAX_BORROW_CAP;
	}

	ord_parameter_types! {
		pub const ZeroAdmin: AccountId = 0;
	}

	impl controller::Config for Test {
		type Event = Event;
		type LiquidityPoolsManager = liquidity_pools::Module<Test>;
		type MaxBorrowCap = MaxBorrowCap;
		type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
		type ControllerWeightInfo = ();
	}

	parameter_types! {
		pub const BlocksPerYear: u128 = 5256000;
	}

	impl minterest_model::Config for Test {
		type Event = Event;
		type BlocksPerYear = BlocksPerYear;
		type ModelUpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
		type WeightInfo = ();
	}

	pub const ADMIN: AccountId = 0;
	pub const ALICE: AccountId = 1;
	pub const BOB: AccountId = 2;
	pub const ONE_HUNDRED: Balance = 100_000 * DOLLARS;
	pub const BALANCE_ZERO: Balance = 0;
	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const RATE_ZERO: Rate = Rate::from_inner(0);
	pub const MAX_BORROW_CAP: Balance = 1_000_000_000_000_000_000_000_000;

	pub fn admin() -> Origin {
		Origin::signed(ADMIN)
	}
	pub fn alice() -> Origin {
		Origin::signed(ALICE)
	}

	pub struct ExtBuilder {
		endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
		pools: Vec<(CurrencyId, Pool)>,
		pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	}

	impl Default for ExtBuilder {
		fn default() -> Self {
			Self {
				endowed_accounts: vec![],
				pools: vec![],
				pool_user_data: vec![],
			}
		}
	}

	impl ExtBuilder {
		pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
			self.endowed_accounts.push((user, currency_id, balance));
			self
		}

		pub fn pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
			self.endowed_accounts
				.push((TestPools::pools_account_id(), currency_id, balance));
			self
		}

		pub fn pool_total_borrowed(mut self, pool_id: CurrencyId, total_borrowed: Balance) -> Self {
			self.pools.push((
				pool_id,
				Pool {
					total_borrowed,
					borrow_index: Rate::one(),
					total_insurance: Balance::zero(),
				},
			));
			self
		}

		pub fn pool_user_data(
			mut self,
			pool_id: CurrencyId,
			user: AccountId,
			total_borrowed: Balance,
			interest_index: Rate,
			collateral: bool,
			liquidation_attempts: u8,
		) -> Self {
			self.pool_user_data.push((
				pool_id,
				user,
				PoolUserData {
					total_borrowed,
					interest_index,
					collateral,
					liquidation_attempts,
				},
			));
			self
		}

		pub fn pool_initial(mut self, pool_id: CurrencyId) -> Self {
			self.pools.push((
				pool_id,
				Pool {
					total_borrowed: Balance::zero(),
					borrow_index: Rate::one(),
					total_insurance: Balance::zero(),
				},
			));
			self
		}

		pub fn build(self) -> sp_io::TestExternalities {
			let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

			orml_tokens::GenesisConfig::<Test> {
				endowed_accounts: self.endowed_accounts,
			}
			.assimilate_storage(&mut t)
			.unwrap();

			controller::GenesisConfig::<Test> {
				controller_dates: vec![
					(
						CurrencyId::DOT,
						ControllerData {
							timestamp: 0,
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
						},
					),
					(
						CurrencyId::ETH,
						ControllerData {
							timestamp: 0,
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
						},
					),
					(
						CurrencyId::BTC,
						ControllerData {
							timestamp: 0,
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
						},
					),
				],
				pause_keepers: vec![
					(
						CurrencyId::ETH,
						PauseKeeper {
							deposit_paused: false,
							redeem_paused: false,
							borrow_paused: false,
							repay_paused: false,
							transfer_paused: false,
						},
					),
					(
						CurrencyId::DOT,
						PauseKeeper {
							deposit_paused: false,
							redeem_paused: false,
							borrow_paused: false,
							repay_paused: false,
							transfer_paused: false,
						},
					),
					(
						CurrencyId::KSM,
						PauseKeeper {
							deposit_paused: true,
							redeem_paused: true,
							borrow_paused: true,
							repay_paused: true,
							transfer_paused: true,
						},
					),
					(
						CurrencyId::BTC,
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

			liquidity_pools::GenesisConfig::<Test> {
				pools: self.pools,
				pool_user_data: self.pool_user_data,
			}
			.assimilate_storage(&mut t)
			.unwrap();

			minterest_model::GenesisConfig {
				minterest_model_dates: vec![
					(
						CurrencyId::DOT,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
					(
						CurrencyId::ETH,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
					(
						CurrencyId::BTC,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
				],
			}
			.assimilate_storage::<Test>(&mut t)
			.unwrap();

			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));
			ext
		}
	}
}
