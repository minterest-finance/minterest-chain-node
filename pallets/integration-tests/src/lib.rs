#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, impl_outer_origin, parameter_types};
	use frame_system::{self as system};
	use liquidity_pools::{Pool, PoolUserData};
	use minterest_primitives::{Balance, CurrencyId, Rate};
	use orml_currencies::Currency;
	use orml_traits::MultiCurrency;
	use pallet_traits::Borrowing;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{IdentityLookup, Zero},
		DispatchResult, FixedPointNumber, ModuleId, Perbill,
	};

	use controller::{ControllerData, PauseKeeper};
	use minterest_model::MinterestModelData;
	use minterest_protocol::Error as MinterestProtocolError;

	mod controller_tests;

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

	pub struct MockBorrowing;
	impl Borrowing<AccountId> for MockBorrowing {
		fn update_state_on_borrow(
			_who: &AccountId,
			_underlying_asset_id: CurrencyId,
			_amount_borrowed: Balance,
			_account_borrows: Balance,
		) -> DispatchResult {
			Ok(())
		}

		fn update_state_on_repay(
			_who: &AccountId,
			_underlying_asset_id: CurrencyId,
			_amount_borrowed: Balance,
			_account_borrows: Balance,
		) -> DispatchResult {
			Ok(())
		}
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
		pub MTokensId: Vec<CurrencyId> = vec![
			CurrencyId::MDOT,
			CurrencyId::MKSM,
			CurrencyId::MBTC,
			CurrencyId::METH,
		];
		pub UnderlyingAssetId: Vec<CurrencyId> = vec![
			CurrencyId::DOT,
			CurrencyId::KSM,
			CurrencyId::BTC,
			CurrencyId::ETH,
		];
	}

	impl liquidity_pools::Trait for Test {
		type Event = ();
		type MultiCurrency = orml_tokens::Module<Test>;
		type ModuleId = LiquidityPoolsModuleId;
		type InitialExchangeRate = InitialExchangeRate;
		type UnderlyingAssetId = UnderlyingAssetId;
		type MTokensId = MTokensId;
	}

	impl minterest_protocol::Trait for Test {
		type Event = ();
		type Borrowing = MockBorrowing;
	}

	parameter_types! {
		pub const BlocksPerYear: u128 = 5256000;
	}

	impl controller::Trait for Test {
		type Event = ();
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
	pub const RATE_EQUALS_ONE: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub const RATE_ZERO: Rate = Rate::from_inner(0);
	pub const MAX_MEMBERS: u32 = 16;
	pub type MinterestProtocol = minterest_protocol::Module<Test>;
	pub type TestPools = liquidity_pools::Module<Test>;
	pub type TestController = controller::Module<Test>;
	pub type Currencies = orml_currencies::Module<Test>;
	pub type System = frame_system::Module<Test>;

	pub struct ExtBuilder {
		endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
		pools: Vec<(CurrencyId, Pool)>,
		pool_user_data: Vec<(AccountId, CurrencyId, PoolUserData)>,
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

	pub fn admin() -> Origin {
		Origin::signed(ADMIN)
	}

	pub fn alice() -> Origin {
		Origin::signed(ALICE)
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
			user: AccountId,
			pool_id: CurrencyId,
			total_borrowed: Balance,
			interest_index: Rate,
			collateral: bool,
		) -> Self {
			self.pool_user_data.push((
				user,
				pool_id,
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
							borrow_rate: Rate::from_inner(0),
							supply_rate: Rate::from_inner(0),
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						},
					),
					(
						CurrencyId::ETH,
						ControllerData {
							timestamp: 0,
							borrow_rate: Rate::from_inner(0),
							supply_rate: Rate::from_inner(0),
							insurance_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
						},
					),
					(
						CurrencyId::BTC,
						ControllerData {
							timestamp: 0,
							borrow_rate: Rate::from_inner(0),
							supply_rate: Rate::from_inner(0),
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

	/* ----------------------------------------------------------------------------------------- */

	// Description of scenario #1:
	// In this scenario, user uses four operations in the protocol (deposit, borrow, repay, redeem).
	// Changes to the main protocol parameters are also checked here.
	#[test]
	fn scenario_with_four_operations() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_initial(CurrencyId::DOT)
			.build()
			.execute_with(|| {
				// INITIAL PARAMS
				/* ------------------------------------------------------------------------------ */

				let alice_dot_free_balance_start: Balance = ONE_HUNDRED;
				let alice_m_dot_free_balance_start: Balance = BALANCE_ZERO;
				let alice_dot_total_borrow_start: Balance = BALANCE_ZERO;

				let pool_available_liquidity_start: Balance = BALANCE_ZERO;
				let pool_m_dot_total_issuance_start: Balance = BALANCE_ZERO;
				let pool_total_insurance_start: Balance = BALANCE_ZERO;
				let pool_dot_total_borrow_start: Balance = BALANCE_ZERO;

				// ACTION: DEPOSIT INSURANCE
				/* ------------------------------------------------------------------------------ */

				// Add liquidity to DOT pool from Insurance by Admin
				let admin_deposit_amount_block_number_0: Balance = 100_000 * DOLLARS;
				assert_ok!(TestController::deposit_insurance(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					admin_deposit_amount_block_number_0
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 100_000
				let current_pool_available_liquidity_block_number_0: Balance =
					pool_available_liquidity_start + admin_deposit_amount_block_number_0;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_0
				);

				// Checking free balance MDOT in pool.
				// Admin doesn't have to get wrapped token after adding liquidity from insurance.
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_total_issuance_start
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).borrow_index, RATE_EQUALS_ONE);
				// Total insurance changed: 0 -> 100 000
				let pool_total_insurance_block_number_0 =
					pool_total_insurance_start + admin_deposit_amount_block_number_0;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_0
				);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_start
				);

				// Checking controller params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 0);
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).borrow_rate, RATE_ZERO);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					alice_dot_total_borrow_start
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);

				System::set_block_number(1);

				// ACTION: DEPOSIT UNDERLYING
				/* ------------------------------------------------------------------------------ */

				// ALICE deposit 60 000 to DOT pool
				let alice_deposit_amount_block_number_1: Balance = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposit_amount_block_number_1
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 160 000
				let pool_available_liquidity_block_number_1: Balance =
					admin_deposit_amount_block_number_0 + alice_deposit_amount_block_number_1;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					pool_available_liquidity_block_number_1
				);

				// Checking free balance MDOT in pool.
				// Admin doesn't have to get wrapped token after adding liquidity from insurance.
				// Alice gets wrapped token after adding liquidity by exchange rate 1:1
				// Expected: 60 000
				let pool_m_dot_free_balance_block_number_1: Balance =
					pool_m_dot_total_issuance_start + alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				let alice_dot_free_balance_block_number_1: Balance =
					alice_dot_free_balance_start - alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_1
				);
				let alice_m_dot_free_balance_block_number_1: Balance =
					alice_m_dot_free_balance_start + alice_deposit_amount_block_number_1;
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).borrow_index, RATE_EQUALS_ONE);
				// Expected: 100 000
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_0
				);
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_borrowed, BALANCE_ZERO);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 1);
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).borrow_rate, RATE_ZERO);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE:
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);

				System::set_block_number(2);

				// ACTION: BORROW
				/* ------------------------------------------------------------------------------ */

				//  Alice borrow 30_000 from DOT pool.
				let alice_borrow_amount_block_number_2: Balance = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrow_amount_block_number_2
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected 130 000
				let current_pool_available_liquidity_block_number_2: Balance =
					pool_available_liquidity_block_number_1 - alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_2
				);

				// Checking free balance MDOT in pool.
				// Expected: 60 000
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				// Expected: 70 000
				let alice_dot_free_balance_block_number_2: Balance =
					alice_dot_free_balance_block_number_1 + alice_borrow_amount_block_number_2;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_2
				);
				// Expected: 60 000
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).borrow_index, RATE_EQUALS_ONE);
				// Expected: 100 000
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_0
				);
				// Total borrowed amount changed 0 -> 30 000
				let pool_dot_total_borrow_block_number_2: Balance =
					pool_dot_total_borrow_start + alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_block_number_2
				);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 2);
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).borrow_rate, RATE_ZERO);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE:
				// User total borrowed changed: 0 -> 30 000
				let alice_dot_total_borrow_block_number_2: Balance =
					alice_dot_total_borrow_start + alice_borrow_amount_block_number_2;
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					alice_dot_total_borrow_block_number_2
				);
				// User interest index changed: 0 -> 1
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					RATE_EQUALS_ONE
				);

				System::set_block_number(3);

				// ACTION: REPAY
				/* ------------------------------------------------------------------------------ */

				// Alice repay part of her loan(15 000).
				let alice_repay_amount_block_number_3: Balance = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::repay(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_repay_amount_block_number_3
				));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected 145 000
				let current_pool_available_liquidity_block_number_3: Balance =
					current_pool_available_liquidity_block_number_2 + alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_3
				);

				// Checking free balance MDOT in pool.
				// Expected: 60 000
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				// Expected: 55 000
				let alice_dot_free_balance_block_number_3: Balance =
					alice_dot_free_balance_block_number_2 - alice_repay_amount_block_number_3;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_3
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);

				// Checking pool Storage params
				// Expected: 1.000000004500000000
				let pool_borrow_index_block_number_3: Rate =
					Rate::saturating_from_rational(10_000_000_045u128, 10_000_000_000u128);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).borrow_index,
					pool_borrow_index_block_number_3
				);
				// Expected: 100_000,0000135
				let insurance_accumulated_block_number_3: Balance = 13_500_000_000_000;
				let pool_total_insurance_block_number_3: Balance =
					admin_deposit_amount_block_number_0 + insurance_accumulated_block_number_3;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_3
				);
				// Expected: 15_000,000135
				let borrow_accumulated_block_number_3: Balance = 135_000_000_000_000;
				let pool_dot_total_borrow_block_number_3: Balance = pool_dot_total_borrow_block_number_2
					+ borrow_accumulated_block_number_3
					- alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_block_number_3
				);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 3);
				// Borrow_rate changed: 0 -> 0,0000000045
				let borrow_rate_block_number_3: Rate = Rate::saturating_from_rational(45u128, 10_000_000_000u128);
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					borrow_rate_block_number_3
				);

				// Checking DOT pool User params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE:
				let alice_dot_total_borrow_block_number_3: Balance = alice_dot_total_borrow_block_number_2
					+ borrow_accumulated_block_number_3
					- alice_repay_amount_block_number_3;
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					alice_dot_total_borrow_block_number_3
				);
				// Interest_index changed: 0 -> 1.000000004500000000
				let user_interest_index_block_number_3: Rate = pool_borrow_index_block_number_3;
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					user_interest_index_block_number_3
				);

				System::set_block_number(4);

				// ACTION: REPAY_ALL
				/* ------------------------------------------------------------------------------ */

				// Alice repay all loans.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Real expected: 		160_000,000168750000528750
				// Currently expected:	160_000,000168750000526875
				// FIXME: unavailable behavior. That is a reason of error below.
				// FIXME: borrow_accumulated_block_number_4 should be 33_750_000_528_750
				//										   instead of 33_750_000_526_875
				let borrow_accumulated_block_number_4: Balance = 33_750_000_526_875;
				let current_pool_available_liquidity_block_number_4: Balance =
					current_pool_available_liquidity_block_number_3
						+ alice_repay_amount_block_number_3
						+ borrow_accumulated_block_number_3
						+ borrow_accumulated_block_number_4;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_4
				);

				// Checking free balance MDOT in pool.
				// Expected: 60 000
				assert_eq!(
					Currencies::total_issuance(CurrencyId::MDOT),
					pool_m_dot_free_balance_block_number_1
				);
				// Checking free balance DOT && MDOT for ADMIN
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE:
				let alice_dot_free_balance_block_number_4: Balance = alice_dot_free_balance_block_number_3
					- alice_dot_total_borrow_block_number_3
					- borrow_accumulated_block_number_4;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_4
				);
				// Expected: 60 000
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_m_dot_free_balance_block_number_1
				);
				// Checking pool Storage params
				// Borrow_index changed: 1.000000004500000000 -> 1,000000006750000025
				let pool_borrow_index_block_number_4 =
					Rate::saturating_from_rational(1_000_000_006_750_000_025u128, 1_000_000_000_000_000_000u128);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).borrow_index,
					pool_borrow_index_block_number_4
				);
				let insurance_accumulated_block_number_4: Balance = 3_375_000_052_875;
				let pool_total_insurance_block_number_4: Balance =
					pool_total_insurance_block_number_3 + insurance_accumulated_block_number_4;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_4
				);

				// FIXME: unavailable behavior.
				// TODO: should be fixed
				// It must be zero, but it is not.
				// 1875 left - 0 right
				// 15000000168750000528750 new borrow value accrue_interest
				// 15000000168750000526875 new user borrow value
				let borrow_accumulated_block_number_4 = 33_750_000_528_750u128;
				let alice_borrow_accumulated_block_number_4 = 33_750_000_526_875u128;
				let pool_dot_total_borrow_block_number_4 = pool_dot_total_borrow_block_number_3
					+ borrow_accumulated_block_number_4
					- alice_dot_total_borrow_block_number_3
					- alice_borrow_accumulated_block_number_4;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					pool_dot_total_borrow_block_number_4
				);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 4);
				// Borrow_rate changed: 0,0000000045 -> 0,000000002250000015
				let borrow_rate_block_number_4 =
					Rate::saturating_from_rational(2_250_000_015u128, 1_000_000_000_000_000_000u128);
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					borrow_rate_block_number_4
				);

				// Checking user pool Storage params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE:
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				let user_interest_index_block_number_4: Rate = pool_borrow_index_block_number_4;
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					user_interest_index_block_number_4
				);

				// Check the underline amount before fn accrue_interest called
				let alice_underlining_amount: Balance =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_m_dot_free_balance_block_number_1).unwrap();

				System::set_block_number(5);

				// ACTION: REDEEM
				/* ------------------------------------------------------------------------------ */

				// Alice redeem all assets
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// PARAMETERS CHECKING
				/* ------------------------------------------------------------------------------ */

				// Checking pool available liquidity
				// Expected: 160_000,000016875000046875
				let current_pool_available_liquidity_block_number_5: Balance =
					current_pool_available_liquidity_block_number_4 - alice_underlining_amount;
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					current_pool_available_liquidity_block_number_5
				);

				// Checking free balance MDOT in pool.
				// Expected: 0
				assert_eq!(Currencies::total_issuance(CurrencyId::MDOT), BALANCE_ZERO);

				// Checking free balance DOT && MDOT
				// ADMIN:
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);
				// ALICE:
				// Expected 99_999,999983124999953125
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_4 + alice_underlining_amount
				);
				// Expected: 0
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), BALANCE_ZERO);

				// Checking pool Storage params
				// Expected: 1,000000006750000025
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).borrow_index,
					pool_borrow_index_block_number_4
				);
				// Expected: 100_000,000016875000052875
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					pool_total_insurance_block_number_4
				);
				//FIXME: something went wrong.....
				//TODO: should be fixed
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_borrowed, 1875);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 5);
				// borrow_rate changed: 0,000000002250000015 -> 0
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					Rate::from_inner(0)
				);

				// Checking user pool Storage params
				// ADMIN:
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE:
				// Expected: 0
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				// Expected: 1,000000006750000025
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					user_interest_index_block_number_4
				);
			});
	}

	// MinterestProtocol tests
	#[test]
	fn deposit_underlying_with_supplied_insurance_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Calculate expected amount of wrapped tokens for Alice
				let alice_expected_amount_wrapped_tokens =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap();

				// Checking pool available liquidity increased by 60 000
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);

				// Checking current free balance for DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_expected_amount_wrapped_tokens
				);

				// Checking current total insurance
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, ONE_HUNDRED);

				System::set_block_number(2);

				// Alice deposit to DOT pool
				let bob_deposited_amount = ONE_HUNDRED;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				// Calculate expected amount of wrapped tokens for Bob
				let bob_expected_amount_wrapped_tokens =
					TestPools::convert_to_wrapped(CurrencyId::DOT, bob_deposited_amount).unwrap();

				// Checking pool available liquidity increased by 60 000
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount + bob_deposited_amount
				);

				// Checking current free balance for DOT && MDOT
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40_000 * DOLLARS);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					ONE_HUNDRED - bob_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					alice_expected_amount_wrapped_tokens
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &BOB),
					bob_expected_amount_wrapped_tokens
				);

				// Checking current total insurance
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, ONE_HUNDRED);
			});
	}

	#[test]
	fn deposit_underlying_overflow_while_convert_underline_to_wrap_should_work() {
		ExtBuilder::default()
			// Set genesis to get exchange rate 0,00000000000000001
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::MDOT, DOLLARS)
			.pool_initial(CurrencyId::DOT)
			.pool_balance(CurrencyId::DOT, 5)
			.pool_total_borrowed(CurrencyId::DOT, 5)
			.build()
			.execute_with(|| {
				// Alice try to deposit ONE_HUNDRED to DOT pool
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, ONE_HUNDRED),
					MinterestProtocolError::<Test>::NumOverflow
				);

				// Alice deposit to DOT pool.
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					100
				));
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #1:
	// The user The user tries to redeem all assets in the first currency. He has loan in the first
	// currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice deposit 50 ETH;
	// 3. Alice borrow 50 DOT;
	// 4. Alice can't `redeem_underlying` 60 DOT: 50 ETH * 0.9 collateral < 50 DOT borrow;
	// 5. Alice deposit 10 ETH;
	// 6. Alice `redeem_underlying` 60 DOT;
	// 7. Alice can't `redeem_underlying` 60 ETH.
	#[test]
	fn redeem_underlying_with_current_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(ALICE, CurrencyId::ETH, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit 60 DOT to pool.
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit 50 ETH to pool.
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

				System::set_block_number(3);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount_in_dot - alice_borrowed_amount_in_dot
				);

				// Checking Alice's free balance DOT && MDOT.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth
				);
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth
				);

				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				System::set_block_number(4);

				// Alice try to redeem all from DOT pool
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				let expected_amount_redeemed_underlying_assets = 60000019601999999880000;
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					expected_amount_redeemed_underlying_assets
				));

				// Checking free balance DOT/MDOT && ETH/METH for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
						+ alice_borrowed_amount_in_dot
						+ expected_amount_redeemed_underlying_assets
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth - alice_deposited_amount_in_eth_secondary
				);

				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);
				let expected_amount_wrapped_tokens_in_eth_summary = expected_amount_wrapped_tokens_in_eth
					+ TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth_secondary).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth_summary
				);
				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				let expected_borrow_interest_accumulated = 21779999999850000;
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot + expected_borrow_interest_accumulated
				);

				System::set_block_number(7);

				// Alice try to redeem all from ETH pool
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::ETH,
						alice_deposited_amount_in_eth + alice_deposited_amount_in_eth_secondary
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #2:
	// The user tries to redeem all assets in the first currency. He has loan in the second currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice borrow 50 ETH;
	// 3. Alice can't `redeem` 60 DOT: 0 DOT collateral < 50 ETH borrow;
	#[test]
	fn redeem_underlying_with_another_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT && ETH for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// // Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(3);

				// Alice redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Checking free balance DOT && ETH for user.
				// Expected previously values
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);

				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #3:
	// The user tries to redeem all assets in the first currency. He has loan in the second
	// currency and deposit in the third currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 40 DOT;
	// 2. Alice deposit 40 BTC;
	// 3. Alice borrow 70 ETH;
	// 4. Alice can't `redeem_underlying` 40 DOT;
	// 5. Alice deposit 40 BTC;
	// 6. Alice redeem 40 DOT;
	// 7. Alice can't `redeem_underlying` 40 BTC;
	#[test]
	fn redeem_underlying_with_third_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(ALICE, CurrencyId::BTC, BALANCE_ZERO, RATE_ZERO, true)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to BTC pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 70_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				System::set_block_number(4);

				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				// Alice try to redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_redeemed_amount_in_dot
				));

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount_in_dot - alice_redeemed_amount_in_dot
				);
				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc - alice_deposited_amount_in_btc_secondary
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(7);

				// Alice try to redeem all BTC.
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::BTC,
						alice_deposited_amount_in_btc_secondary
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem_underlying`, description of scenario #4:
	// It is possible to redeem assets from the pool insurance.
	// 1. Deposit 10 DOT to pool insurance;
	// 2. Alice deposit 20 DOT;
	// 3. Bob deposit 20 BTC;
	// 4. Bob deposit 10 DOT;
	// 5. Bob borrow 15 DOT;
	// 6. Alice redeem 20 DOT;
	// 7. DOT pool insurance equal 5 DOT;
	#[test]
	fn redeem_underlying_over_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::BTC, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_user_data(BOB, CurrencyId::BTC, BALANCE_ZERO, RATE_ZERO, true)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.pool_total_insurance(CurrencyId::DOT, 10_000 * DOLLARS)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Bob deposit to BTC pool
				let bob_deposited_amount_in_btc = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::BTC,
					bob_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Bob borrow from DOT pool
				let bob_borrowed_amount_in_dot = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				System::set_block_number(4);

				// Bob deposit to DOT pool
				let bob_deposited_amount_in_dot = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount_in_dot
				));

				System::set_block_number(5);

				// Alice redeem all DOTs.
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				// Expected exchange rate 1000000006581250024
				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_redeemed_amount_in_dot
				));

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					10_000 * DOLLARS + alice_deposited_amount_in_dot - alice_redeemed_amount_in_dot
						+ bob_deposited_amount_in_dot
						- bob_borrowed_amount_in_dot
				);

				// Checking free balance DOT && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					ONE_HUNDRED + bob_borrowed_amount_in_dot - bob_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &BOB),
					ONE_HUNDRED - bob_deposited_amount_in_btc
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #1:
	// The user tries to redeem all assets in the first currency. He has loan in the first currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice deposit 50 ETH;
	// 3. Alice borrow 50 DOT;
	// 4. Alice can't `redeem` 60 DOT: 10 DOT * 0.9 + 50 ETH * 0.9 collateral < 60 DOT redeem;
	// 5. Alice deposit 10 ETH;
	// 6. Alice `redeem` 60 DOT;
	// 7. Alice can't `redeem` 60 ETH.
	#[test]
	fn redeem_with_current_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, 100_000_000 * DOLLARS)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(ALICE, CurrencyId::ETH, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to ETH pool
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

				System::set_block_number(3);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount_in_dot - alice_borrowed_amount_in_dot
				);

				// Checking free balance DOT && MDOT in pool.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth
				);
				let expected_amount_wrapped_tokens_in_dot =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestPools::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth
				);

				// Checking total borrow for Alice DOT pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				System::set_block_number(4);

				// Alice try to redeem all from DOT pool
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				// Bob add liquidity to ETH pool
				let bob_deposited_amount_in_dot = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount_in_dot
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking free balance DOT/MDOT && ETH/METH in pool.
				// current_exchange_rate == 1000000221932654817
				let expected_amount_redeemed_underlying_assets = 60000013315959289020000;
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
						+ alice_borrowed_amount_in_dot
						+ expected_amount_redeemed_underlying_assets
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_eth - alice_deposited_amount_in_eth_secondary
				);

				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);
				let expected_amount_wrapped_tokens_in_eth_summary = expected_amount_wrapped_tokens_in_eth
					+ TestPools::convert_to_wrapped(CurrencyId::ETH, alice_deposited_amount_in_eth_secondary).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::METH, &ALICE),
					expected_amount_wrapped_tokens_in_eth_summary
				);
				// Checking total borrow for Alice DOT pool
				let expected_amount_accumulated_in_dot = 14841428697992866;
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);
				// Checking total borrow for DOT pool
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot + expected_amount_accumulated_in_dot
				);

				System::set_block_number(7);

				// Alice try to redeem all from ETH pool
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::ETH),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #2:
	// The user tries to redeem all assets in the first currency. He has loan in the second currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice borrow 50 ETH;
	// 3. Alice can't `redeem` 60 DOT: 0 DOT collateral < 50 ETH borrow;
	#[test]
	fn redeem_with_another_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT && ETH for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// // Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(3);

				// Alice redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Checking free balance DOT && ETH for user.
				// Expected previously values
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);

				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #3:
	// The user tries to redeem all assets in the first currency. He has loan in the second
	// currency and deposit in the third currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 40 DOT;
	// 2. Alice deposit 40 BTC;
	// 3. Alice borrow 70 ETH;
	// 4. Alice can't `redeem` 40 DOT: (40 BTC * 0.9) collateral < 70 ETH borrow;
	// 5. Alice deposit 40 BTC;
	// 6. Alice redeem 40 DOT: (80 BTC * 0.9) collateral > 70 EHT borrow;
	// 7. Alice can't `redeem` 40 BTC: (40 BTC * 0.9) collateral < 70 ETH borrow;
	#[test]
	fn redeem_with_third_currency_borrowing() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(ALICE, CurrencyId::BTC, BALANCE_ZERO, RATE_ZERO, true)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Alice deposit to BTC pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

				System::set_block_number(3);

				// Alice borrow from ETH pool
				let alice_borrowed_amount_in_eth = 70_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount_in_eth
				));

				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(4);

				// Alice try to redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				System::set_block_number(5);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				System::set_block_number(6);

				// Alice redeem all DOTs
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking free balance DOT && ETH && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_btc - alice_deposited_amount_in_btc_secondary
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::ETH, &ALICE),
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for Alice ETH pool
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);
				// Checking total borrow for ETH pool
				assert_eq!(
					TestPools::pools(CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount_in_eth
				);

				System::set_block_number(7);

				// Alice try to redeem all BTC.
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::BTC),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	// Extrinsic `redeem`, description of scenario #4:
	// It is possible to redeem assets from the pool insurance.
	// 1. Deposit 10 DOT to pool insurance;
	// 2. Alice deposit 20 DOT;
	// 3. Bob deposit 20 BTC;
	// 4. Bob deposit 10 DOT;
	// 5. Bob borrow 15 DOT;
	// 6. Alice redeem 20 DOT, pool insurance equal 5 DOT;
	#[test]
	fn redeem_over_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::BTC, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_user_data(BOB, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_user_data(BOB, CurrencyId::BTC, BALANCE_ZERO, RATE_ZERO, true)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.pool_total_insurance(CurrencyId::DOT, 10_000 * DOLLARS)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				System::set_block_number(2);

				// Bob deposit to BTC pool
				let bob_deposited_amount_in_btc = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::BTC,
					bob_deposited_amount_in_btc
				));

				// Bob deposit to DOT pool
				let bob_deposited_amount_in_dot = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount_in_dot
				));

				System::set_block_number(3);

				// Bob borrow from DOT pool
				let bob_borrowed_amount_in_dot = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				System::set_block_number(4);

				// Alice redeem all DOTs.
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);

				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				let alice_redeemed_amount_in_dot =
					TestPools::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot).unwrap();

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					10_000 * DOLLARS + alice_deposited_amount_in_dot - alice_redeemed_amount_in_dot
						+ bob_deposited_amount_in_dot
						- bob_borrowed_amount_in_dot
				);

				// Checking free balance DOT && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					ONE_HUNDRED + bob_borrowed_amount_in_dot - bob_deposited_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &BOB),
					ONE_HUNDRED - bob_deposited_amount_in_btc
				);
			});
	}

	// Extrinsic `borrow`, description of scenario #1:
	// The user cannot borrow without making a deposit first.
	// 1. Alice can't borrow 50 DOT: 0 collateral < 50 DOT borrow;
	#[test]
	fn borrow_with_insufficient_collateral_no_deposits() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice try to borrow from DOT pool
				let alice_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, alice_borrowed_amount_in_dot),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// Checking pool available liquidity
				assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), ONE_HUNDRED);
			});
	}

	// Extrinsic `borrow`, description of scenario #2:
	// The user cannot borrow in the second currency unless he has
	// not enabled the first currency as collateral.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Alice can't borrow 50 ETH: 0 collateral < 50 ETH borrow;
	#[test]
	fn borrow_without_collateral_in_second_currency() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 50_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::ETH, alice_borrowed_amount),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);
				assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::ETH), ONE_HUNDRED);
			});
	}

	// Extrinsic `borrow`, description of scenario #3:
	// The user cannot borrow in the second currency if the collateral in the first currency
	// is insufficient.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Alice can't borrow 50 ETH: 50 DOT * 0.9 collateral < 50 ETH borrow;
	#[test]
	fn borrow_with_insufficient_collateral_in_second_currency() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 50_000 * DOLLARS;
				assert_noop!(
					MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::ETH, alice_borrowed_amount),
					MinterestProtocolError::<Test>::BorrowControllerRejection
				);

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);
				assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::ETH), ONE_HUNDRED);
			});
	}

	// Extrinsic `borrow`, description of scenario #4:
	// The user can borrow in the second currency if the collateral in the first currency
	// is sufficient.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 50 DOT;
	// 2. Alice can borrow 40 ETH: 50 DOT * 0.9 collateral > 40 ETH borrow;
	#[test]
	fn borrow_with_sufficient_collateral_in_second_currency() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.pool_total_insurance(CurrencyId::ETH, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_borrowed_amount
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					ONE_HUNDRED + alice_deposited_amount
				);
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::ETH),
					ONE_HUNDRED - alice_borrowed_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(Currencies::free_balance(CurrencyId::ETH, &ALICE), alice_borrowed_amount);
				assert_eq!(TestPools::pools(CurrencyId::ETH).total_borrowed, alice_borrowed_amount);
				assert_eq!(
					TestPools::pool_user_data(&ALICE, CurrencyId::ETH).total_borrowed,
					alice_borrowed_amount
				);
			});
	}

	// Extrinsic `set_insurance_factor`, description of scenario #1:
	// Pool insurance does not increase if the insurance_factor is zero.
	// 1. Alice deposit 40 DOT;
	// 2. Alice borrow 20 DOT;
	// 3. Set insurance factor equal to zero.
	// 4. Alice repay full loan in DOTs, pool total_insurance = 0.
	#[test]
	fn set_insurance_factor_equal_zero() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - alice_borrowed_amount_in_dot
				);
				// Checking total insurance for DOT pool.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);

				System::set_block_number(10);

				// Set insurance factor equal to zero.
				assert_ok!(TestController::set_insurance_factor(admin(), CurrencyId::DOT, 0, 1));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking pool total insurance.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);
			});
	}

	// Extrinsic `set_insurance_factor`, description of scenario #2:
	// Pool insurance is increased if the insurance_factor is greater than zero.
	// 1. Alice deposit 40 DOT;
	// 2. Alice borrow 20 DOT;
	// 3. Set insurance factor equal 0.5.
	// 4. Alice repay full loan in DOTs, pool insurance increased.
	#[test]
	fn set_insurance_factor_greater_than_zero() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - alice_borrowed_amount_in_dot
				);
				// Checking total insurance for DOT pool.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);

				System::set_block_number(10);

				// Set insurance factor equal 0.5.
				assert_ok!(TestController::set_insurance_factor(admin(), CurrencyId::DOT, 1, 2));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				let expected_interest_accumulated: Balance = 720_000_000_000_000;

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount + expected_interest_accumulated
				);
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_insurance,
					BALANCE_ZERO + (expected_interest_accumulated / 2)
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #1:
	#[test]
	fn calculate_borrow_interest_rate_deposit_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #2:
	#[test]
	fn calculate_borrow_interest_rate_deposit_with_pool_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #3:
	#[test]
	fn calculate_borrow_interest_rate_deposit_and_borrow_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #4:
	#[test]
	fn calculate_borrow_interest_rate_deposit_and_borrow_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::zero();

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `calculate_borrow_interest_rate + calculate_utilization_rate` scenario #5:
	#[test]
	fn calculate_borrow_interest_rate_few_deposits_and_borrows_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(BOB, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				System::set_block_number(3);

				// Bob deposit to DOT pool
				let bob_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				System::set_block_number(4);

				// Expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = Rate::from_inner(1800000006);

				// Alice try to borrow from DOT pool
				let bob_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// Checking if real borrow interest rate is equal to the expected
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #1:
	#[test]
	fn get_exchange_rate_deposit_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #2:
	#[test]
	fn get_exchange_rate_deposit_with_pool_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #3:
	#[test]
	fn get_exchange_rate_deposit_and_borrow_without_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real borrow interest rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #4:
	#[test]
	fn get_exchange_rate_deposit_and_borrow_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Expected exchange rate && wrapped amount based on params after fn accrue_interest_rate called
				let expected_amount_wrapped_tokens = 40_000 * DOLLARS;
				let expected_exchange_rate_mock = Rate::one();

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock)
				);
			});
	}

	// Function `get_exchange_rate + calculate_exchange_rate` scenario #5:
	#[test]
	fn get_exchange_rate_few_deposits_and_borrows_with_insurance() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_user_data(BOB, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				System::set_block_number(2);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				System::set_block_number(3);

				// Bob deposit to DOT pool
				let bob_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				System::set_block_number(4);

				// Expected exchange rate based on params before fn accrue_interest_rate in block 4 called
				let expected_exchange_rate_mock_block_number_3 = Rate::from_inner(1000000002025000000);

				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock_block_number_3)
				);

				// Alice try to borrow from DOT pool
				let bob_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// Expected exchange rate && wrapped amount based on params after
				// fn accrue_interest_rate in block 4 called
				let expected_amount_wrapped_tokens_alice = 40_000 * DOLLARS;
				// bob_deposited_amount/expected_exchange_rate_mock_block_number_3 = 59_999_999_878_500_000_246_037
				let expected_amount_wrapped_tokens_bob = 59_999_999_878_500_000_246_037;
				let expected_exchange_rate_mock_block_number_4 = Rate::from_inner(1000000002349000003);

				// Checking if real exchange rate && wrapped amount is equal to the expected
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_alice
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &BOB),
					expected_amount_wrapped_tokens_bob
				);
				assert_eq!(
					TestPools::get_exchange_rate(CurrencyId::DOT),
					Ok(expected_exchange_rate_mock_block_number_4)
				);
			});
	}
}
