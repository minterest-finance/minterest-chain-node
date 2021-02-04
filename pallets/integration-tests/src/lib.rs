#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, impl_outer_origin, parameter_types};
	use frame_system::{self as system};
	use liquidity_pools::{Pool, PoolUserData};
	use minterest_primitives::{Balance, CurrencyId, CurrencyPair, Rate};
	use orml_currencies::Currency;
	use orml_traits::MultiCurrency;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{IdentityLookup, Zero},
		FixedPointNumber, ModuleId, Perbill,
	};

	use controller::{ControllerData, PauseKeeper};
	use minterest_model::MinterestModelData;
	use minterest_protocol::Error as MinterestProtocolError;
	use pallet_traits::PoolsManager;

	mod controller_tests;
	mod liquidity_pools_tests;
	mod minterest_model_tests;
	mod minterest_protocol_tests;
	mod scenario_tests;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	#[derive(Clone, PartialEq, Eq)]
	pub struct Test;

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: u32 = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::one();

	}

	pub type AccountId = u32;
	impl system::Trait for Test {
		type BaseCallFilter = ();
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = ::sp_runtime::traits::BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type DbWeight = ();
		type BlockExecutionWeight = ();
		type ExtrinsicBaseWeight = ();
		type MaximumExtrinsicWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type PalletInfo = ();
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
	}

	parameter_types! {
		pub const ExistentialDeposit: u64 = 1;
	}

	type Amount = i128;
	impl orml_tokens::Trait for Test {
		type Event = ();
		type Balance = Balance;
		type Amount = Amount;
		type CurrencyId = CurrencyId;
		type OnReceived = ();
		type WeightInfo = ();
	}

	parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::MINT;
	}

	type NativeCurrency = Currency<Test, GetNativeCurrencyId>;

	impl orml_currencies::Trait for Test {
		type Event = ();
		type MultiCurrency = orml_tokens::Module<Test>;
		type NativeCurrency = NativeCurrency;
		type GetNativeCurrencyId = GetNativeCurrencyId;
		type WeightInfo = ();
	}

	impl m_tokens::Trait for Test {
		type Event = ();
		type MultiCurrency = orml_tokens::Module<Test>;
	}

	parameter_types! {
		pub const LiquidityPoolsModuleId: ModuleId = ModuleId(*b"min/pool");
		pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
		pub EnabledCurrencyPair: Vec<CurrencyPair> = vec![
			CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT),
			CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM),
			CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC),
			CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH),
		];
	}

	impl liquidity_pools::Trait for Test {
		type Event = ();
		type MultiCurrency = orml_tokens::Module<Test>;
		type ModuleId = LiquidityPoolsModuleId;
		type InitialExchangeRate = InitialExchangeRate;
		type EnabledCurrencyPair = EnabledCurrencyPair;
	}

	impl minterest_protocol::Trait for Test {
		type Event = ();
		type Borrowing = liquidity_pools::Module<Test>;
		type ManagerLiquidityPools = liquidity_pools::Module<Test>;
	}

	impl controller::Trait for Test {
		type Event = ();
		type LiquidityPoolsManager = liquidity_pools::Module<Test>;
	}

	impl oracle::Trait for Test {
		type Event = ();
	}

	parameter_types! {
		pub const MaxMembers: u32 = MAX_MEMBERS;
	}

	impl accounts::Trait for Test {
		type Event = ();
		type MaxMembers = MaxMembers;
	}

	parameter_types! {
		pub const BlocksPerYear: u128 = 5256000;
	}

	impl minterest_model::Trait for Test {
		type Event = ();
		type BlocksPerYear = BlocksPerYear;
	}

	pub const ADMIN: AccountId = 0;
	pub const ALICE: AccountId = 1;
	pub const BOB: AccountId = 2;
	pub const ONE_HUNDRED: Balance = 100_000 * DOLLARS;
	pub const BALANCE_ZERO: Balance = 0;
	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const RATE_ZERO: Rate = Rate::from_inner(0);
	pub const MAX_MEMBERS: u32 = 16;

	pub type MinterestProtocol = minterest_protocol::Module<Test>;
	pub type TestPools = liquidity_pools::Module<Test>;
	pub type TestController = controller::Module<Test>;
	pub type Currencies = orml_currencies::Module<Test>;
	pub type System = frame_system::Module<Test>;

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
					borrow_index: Rate::saturating_from_rational(1, 1),
					total_insurance: Balance::zero(),
				},
			));
			self
		}

		pub fn pool_total_insurance(mut self, pool_id: CurrencyId, total_insurance: Balance) -> Self {
			self.endowed_accounts
				.push((TestPools::pools_account_id(), pool_id, total_insurance));
			self.pools.push((
				pool_id,
				Pool {
					total_borrowed: Balance::zero(),
					borrow_index: Rate::saturating_from_rational(1, 1),
					total_insurance,
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
		) -> Self {
			self.pool_user_data.push((
				pool_id,
				user,
				PoolUserData {
					total_borrowed,
					interest_index,
					collateral,
				},
			));
			self
		}

		pub fn pool_initial(mut self, pool_id: CurrencyId) -> Self {
			self.pools.push((
				pool_id,
				Pool {
					total_borrowed: Balance::zero(),
					borrow_index: Rate::saturating_from_rational(1, 1),
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
						},
					),
					(
						CurrencyId::ETH,
						ControllerData {
							timestamp: 0,
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						},
					),
					(
						CurrencyId::BTC,
						ControllerData {
							timestamp: 0,
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
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
						},
					),
					(
						CurrencyId::DOT,
						PauseKeeper {
							deposit_paused: false,
							redeem_paused: false,
							borrow_paused: false,
							repay_paused: false,
						},
					),
					(
						CurrencyId::KSM,
						PauseKeeper {
							deposit_paused: true,
							redeem_paused: true,
							borrow_paused: true,
							repay_paused: true,
						},
					),
					(
						CurrencyId::BTC,
						PauseKeeper {
							deposit_paused: false,
							redeem_paused: false,
							borrow_paused: false,
							repay_paused: false,
						},
					),
				],
			}
			.assimilate_storage(&mut t)
			.unwrap();

			liquidity_pools::GenesisConfig::<Test> {
				pools: self.pools,
				pool_user_data: self.pool_user_data,
			}
			.assimilate_storage(&mut t)
			.unwrap();

			accounts::GenesisConfig::<Test> {
				allowed_accounts: vec![(ADMIN, ())],
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
			.assimilate_storage(&mut t)
			.unwrap();

			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));
			ext
		}
	}
}
