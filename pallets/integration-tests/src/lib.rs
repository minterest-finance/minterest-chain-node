#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, ensure, impl_outer_origin, parameter_types};
	use frame_system::{self as system};
	use liquidity_pools::{Pool, PoolUserData};
	use minterest_primitives::{Balance, CurrencyId, Operation, Rate};
	use orml_currencies::Currency;
	use orml_traits::MultiCurrency;
	use pallet_traits::Borrowing;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{IdentityLookup, Zero},
		ModuleId, Perbill,
	};
	use sp_runtime::{DispatchError, DispatchResult, FixedPointNumber};

	use controller::{ControllerData, Error as ControllerError, PauseKeeper};
	use minterest_protocol::Error as MinterestProtocolError;
	use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul};
	use sp_std::{cmp::Ordering, result};

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
		pub UnderlyingAssetId: Vec<CurrencyId> = vec![
			CurrencyId::DOT,
			CurrencyId::KSM,
			CurrencyId::BTC,
			CurrencyId::ETH,
		];
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
	}

	impl liquidity_pools::Trait for Test {
		type Event = ();
		type MultiCurrency = orml_tokens::Module<Test>;
		type ModuleId = LiquidityPoolsModuleId;
	}

	impl minterest_protocol::Trait for Test {
		type Event = ();
		type Borrowing = MockBorrowing;
	}

	parameter_types! {
		pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
		pub const BlocksPerYear: u128 = 5256000;
		pub MTokensId: Vec<CurrencyId> = vec![
			CurrencyId::MDOT,
			CurrencyId::MKSM,
			CurrencyId::MBTC,
			CurrencyId::METH,
		];
	}

	impl controller::Trait for Test {
		type Event = ();
		type InitialExchangeRate = InitialExchangeRate;
		type BlocksPerYear = BlocksPerYear;
		type UnderlyingAssetId = UnderlyingAssetId;
		type MTokensId = MTokensId;
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

	pub const ADMIN: AccountId = 0;
	pub const ALICE: AccountId = 1;
	pub const BOB: AccountId = 2;
	pub const ONE_MILL: Balance = 1_000_000;
	pub const ONE_HUNDRED: Balance = 100_000 * DOLLARS;
	pub const BALANCE_ZERO: Balance = 0;
	pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
	pub const RATE_EQUALS_ONE: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	pub const RATE_ZERO: Rate = Rate::from_inner(0);
	pub const MAX_MEMBERS: u32 = 16;
	pub type MinterestProtocol = minterest_protocol::Module<Test>;
	pub type TestPools = liquidity_pools::Module<Test>;
	pub type TestController = controller::Module<Test>;
	pub type TestAccounts = accounts::Module<Test>;
	pub type Currencies = orml_currencies::Module<Test>;
	pub type System = frame_system::Module<Test>;
	type RateResult = result::Result<Rate, DispatchError>;

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
	pub fn bob() -> Origin {
		Origin::signed(BOB)
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
					current_exchange_rate: Rate::from_inner(1),
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
					current_exchange_rate: Rate::from_inner(1),
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
					current_exchange_rate: Rate::saturating_from_rational(1, 1),
					total_insurance: Balance::zero(),
				},
			));
			self
		}

		pub fn alice_deposit_20_eth(self) -> Self {
			self.user_balance(ALICE, CurrencyId::ETH, 80)
				.user_balance(ALICE, CurrencyId::METH, 20)
				.pool_balance(CurrencyId::ETH, 20)
				.pool_user_data(ALICE, CurrencyId::ETH, 0, Rate::from_inner(0), true)
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
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::from_inner(0),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
							collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
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
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::from_inner(0),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
							collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
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
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::from_inner(0),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
							collateral_factor: Rate::saturating_from_rational(9, 10),               // 90%
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

			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));
			ext
		}
	}

	// Mock functions

	pub fn calculate_borrow_interest_rate_mock(underlying_asset_id: CurrencyId) -> RateResult {
		let current_total_balance: Balance = TestPools::get_pool_available_liquidity(underlying_asset_id);
		let current_total_borrowed_balance: Balance = TestPools::pools(underlying_asset_id).total_borrowed;
		let current_total_insurance: Balance = TestPools::pools(underlying_asset_id).total_insurance;

		let utilization_rate = calculate_utilization_rate(
			current_total_balance,
			current_total_borrowed_balance,
			current_total_insurance,
		)?;

		let kink = TestController::controller_dates(underlying_asset_id).kink;
		let multiplier_per_block = TestController::controller_dates(underlying_asset_id).multiplier_per_block;
		let base_rate_per_block = TestController::controller_dates(underlying_asset_id).base_rate_per_block;

		let borrow_interest_rate = match utilization_rate.cmp(&kink) {
			Ordering::Greater => {
				let jump_multiplier_per_block =
					TestController::controller_dates(underlying_asset_id).jump_multiplier_per_block;
				let normal_rate = kink
					.checked_mul(&multiplier_per_block)
					.ok_or(ControllerError::<Test>::NumOverflow)?
					.checked_add(&base_rate_per_block)
					.ok_or(ControllerError::<Test>::NumOverflow)?;
				let excess_util = utilization_rate
					.checked_mul(&kink)
					.ok_or(ControllerError::<Test>::NumOverflow)?;

				excess_util
					.checked_mul(&jump_multiplier_per_block)
					.ok_or(ControllerError::<Test>::NumOverflow)?
					.checked_add(&normal_rate)
					.ok_or(ControllerError::<Test>::NumOverflow)?
			}
			_ => utilization_rate
				.checked_mul(&multiplier_per_block)
				.ok_or(ControllerError::<Test>::NumOverflow)?
				.checked_add(&base_rate_per_block)
				.ok_or(ControllerError::<Test>::NumOverflow)?,
		};

		Ok(borrow_interest_rate)
	}

	/// Calculates the utilization rate of the pool:
	/// utilization_rate = total_borrows / (total_cash + total_borrows - total_insurance)
	fn calculate_utilization_rate(
		current_total_balance: Balance,
		current_total_borrowed_balance: Balance,
		current_total_insurance: Balance,
	) -> RateResult {
		if current_total_borrowed_balance == 0 {
			return Ok(Rate::from_inner(0));
		}

		let total_balance_total_borrowed_sum = current_total_balance
			.checked_add(current_total_borrowed_balance)
			.ok_or(ControllerError::<Test>::NumOverflow)?;
		let denominator = total_balance_total_borrowed_sum
			.checked_sub(current_total_insurance)
			.ok_or(ControllerError::<Test>::NumOverflow)?;

		ensure!(denominator > 0, ControllerError::<Test>::NumOverflow);

		let utilization_rate = Rate::saturating_from_rational(current_total_borrowed_balance, denominator);

		Ok(utilization_rate)
	}

	/* ----------------------------------------------------------------------------------------- */

	// Integration tests.
	#[test]
	fn scenario_should_work() {
		ExtBuilder::default()
			.user_balance(ADMIN, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_initial(CurrencyId::DOT)
			.build()
			.execute_with(|| {
				//FIXME: add checking is operation paused. Currently all operations are not paused.

				// System starts from block number 0.

				// Checking user available liquidity
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::ETH, 10 * DOLLARS),
					MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
				);

				// Checking all invalid underlying assets (MDOT, MKSM, MBTC, METH).
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 10 * DOLLARS),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MKSM, 10 * DOLLARS),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MBTC, 10 * DOLLARS),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::METH, 10 * DOLLARS),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);

				// Initial params
				let alice_dot_free_balance_start: Balance = 100_000 * DOLLARS;
				let alice_m_dot_free_balance_start: Balance = BALANCE_ZERO;
				let alice_dot_total_borrow_start: Balance = BALANCE_ZERO;

				let pool_available_liquidity_start: Balance = BALANCE_ZERO;
				let pool_m_dot_total_issuance_start: Balance = BALANCE_ZERO;
				let pool_total_insurance_start: Balance = BALANCE_ZERO;
				let pool_dot_total_borrow_start: Balance = BALANCE_ZERO;

				// Add liquidity to DOT pool from Insurance by Admin
				let admin_deposit_amount_block_number_0: Balance = 100_000 * DOLLARS;
				assert_ok!(TestController::deposit_insurance(
					Origin::signed(ADMIN),
					CurrencyId::DOT,
					admin_deposit_amount_block_number_0
				));

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
				// ADMIN
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// Checking DOT pool Storage params
				assert_eq!(TestPools::pools(CurrencyId::DOT).current_exchange_rate, RATE_EQUALS_ONE);
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
				// ADMIN
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					alice_dot_total_borrow_start
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);

				// Set next block number
				System::set_block_number(1);

				// ALICE deposit 60 000 to DOT pool
				let alice_deposit_amount_block_number_1: Balance = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposit_amount_block_number_1
				));

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
				// ADMIN
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE
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
				assert_eq!(TestPools::pools(CurrencyId::DOT).current_exchange_rate, RATE_EQUALS_ONE);
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
				// ADMIN
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);

				// Set next block number
				System::set_block_number(2);

				//  Alice borrow 30_000 from DOT pool.
				let alice_borrow_amount_block_number_2: Balance = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrow_amount_block_number_2
				));

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
				// ADMIN
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE
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
				assert_eq!(TestPools::pools(CurrencyId::DOT).current_exchange_rate, RATE_EQUALS_ONE);
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
				// ADMIN
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// ALICE
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

				// Set block number 3
				System::set_block_number(3);

				// Alice repay part of her loan(15 000).
				let alice_repay_amount_block_number_3: Balance = 15_000 * DOLLARS;
				assert_ok!(MinterestProtocol::repay(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_repay_amount_block_number_3
				));

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
				// ADMIN
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);

				// ALICE
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
				assert_eq!(TestPools::pools(CurrencyId::DOT).current_exchange_rate, RATE_EQUALS_ONE);
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
				// Admin
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// Alice
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

				// Set next block number
				System::set_block_number(4);

				// Alice repay all loans.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

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
				// ADMIN
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);

				// ALICE
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
				assert_eq!(TestPools::pools(CurrencyId::DOT).current_exchange_rate, RATE_EQUALS_ONE);
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

				// It must be zero, but it is not. FIXME: unavailable behavior.
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
				// Admin
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// Alice
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				let user_interest_index_block_number_4: Rate = pool_borrow_index_block_number_4;
				assert_eq!(
					TestPools::pool_user_data(ALICE, CurrencyId::DOT).interest_index,
					user_interest_index_block_number_4
				);

				// Set next block number
				System::set_block_number(5);

				// Check the underline amount before fn accrue_interest called
				let alice_underlining_amount: Balance =
					TestController::convert_from_wrapped(CurrencyId::MDOT, alice_m_dot_free_balance_block_number_1)
						.unwrap();

				// Alice redeem all assets
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

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
				// ADMIN
				assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), BALANCE_ZERO);
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), BALANCE_ZERO);
				// Alice
				// Expected 99_999,999983124999953125
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					alice_dot_free_balance_block_number_4 + alice_underlining_amount
				);
				// Expected: 0
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), BALANCE_ZERO);

				// Checking pool Storage params
				// Expected: 1,000000002531250008
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).current_exchange_rate,
					Rate::from_inner(1_000_000_002_531_250_008)
				);
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
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_borrowed, 1875);

				// Checking controller Storage params
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 5);
				// borrow_rate changed: 0,000000002250000015 -> 0
				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					Rate::from_inner(0)
				);

				// Checking user pool Storage params
				// Admin
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).total_borrowed,
					BALANCE_ZERO
				);
				assert_eq!(
					TestPools::pool_user_data(ADMIN, CurrencyId::DOT).interest_index,
					RATE_ZERO
				);
				// Alice
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
	fn deposit_underlying_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_initial(CurrencyId::DOT)
			.build()
			.execute_with(|| {
				// Alice try to deposit unavailable asset.
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 10_000 * DOLLARS),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);

				// Alice try to deposit zero.
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, BALANCE_ZERO),
					MinterestProtocolError::<Test>::ZeroBalanceTransaction
				);

				// Alice try to deposit ETH. Alice ETH balance == 0
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::ETH, 10_000 * DOLLARS),
					MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
				);

				// Checking last accrued block number
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 0);

				// Jump to 10 blocks
				System::set_block_number(10);

				// Alice deposit to DOT pool.
				let alice_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Calculate expected amount of wrapped tokens
				let expected_amount_wrapped_tokens =
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap();

				// Checking last accrued block number have been changed.
				// Expected: 10
				assert_eq!(TestController::controller_dates(CurrencyId::DOT).timestamp, 10);

				// Checking pool available liquidity increased by 60 000
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount
				);

				// Checking current free balance for DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);

				// Alice try to deposit DOT amount grater than she has.
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, 50_000 * DOLLARS),
					MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
				);

				// Admin paused deposit operation for DOT pool.
				assert_ok!(TestController::pause_specific_operation(
					admin(),
					CurrencyId::DOT,
					Operation::Deposit
				));

				// Alice try to deposit some amount of DOT to pool
				assert_noop!(
					MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, 30_000 * DOLLARS),
					MinterestProtocolError::<Test>::DepositControllerRejection
				);

				// Checking pool available liquidity didn't increased
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount
				);
				// Checking Alice's free balance for DOT && MDOT didn't increased.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
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
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap();

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

				// Alice deposit to DOT pool
				let bob_deposited_amount = ONE_HUNDRED;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				// Calculate expected amount of wrapped tokens for Bob
				let bob_expected_amount_wrapped_tokens =
					TestController::convert_to_wrapped(CurrencyId::DOT, bob_deposited_amount).unwrap();

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

	#[test]
	fn redeem_wrapped_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_initial(CurrencyId::DOT)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount
				);

				// Checking free balance DOT && MDOT in pool.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				let expected_amount_wrapped_tokens =
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);

				// Admin pause redeem operation.
				assert_ok!(TestController::pause_specific_operation(
					admin(),
					CurrencyId::DOT,
					Operation::Redeem
				));

				// Alice try to redeem unavailable asset
				assert_noop!(
					MinterestProtocol::redeem_wrapped(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						expected_amount_wrapped_tokens
					),
					MinterestProtocolError::<Test>::NotValidWrappedTokenId
				);

				// Alice try to redeem. Redeem is paused
				assert_noop!(
					MinterestProtocol::redeem_wrapped(
						Origin::signed(ALICE),
						CurrencyId::MDOT,
						expected_amount_wrapped_tokens
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Admin unpause redeem operation.
				assert_ok!(TestController::unpause_specific_operation(
					admin(),
					CurrencyId::DOT,
					Operation::Redeem
				));

				// Alice redeem from DOT pool
				assert_ok!(MinterestProtocol::redeem_wrapped(
					Origin::signed(ALICE),
					CurrencyId::MDOT,
					expected_amount_wrapped_tokens
				));

				// Checking pool available liquidity
				let expected_amount_underlying_assets =
					TestController::convert_from_wrapped(CurrencyId::MDOT, expected_amount_wrapped_tokens).unwrap();
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - expected_amount_underlying_assets
				);

				// Checking free balance DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount + expected_amount_underlying_assets
				);
				// Expected 0
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);

				// Alice try to redeem. MDOT Balance is zero.
				assert_noop!(
					MinterestProtocol::redeem_wrapped(Origin::signed(ALICE), CurrencyId::MDOT, BALANCE_ZERO),
					MinterestProtocolError::<Test>::ZeroBalanceTransaction
				);
			});
	}

	#[test]
	fn redeem_underlying_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_initial(CurrencyId::DOT)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount
				);

				// Checking free balance DOT && MDOT in pool.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap()
				);

				// Alice try to redeem overbalance
				assert_noop!(
					MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::DOT, 100_000 * DOLLARS),
					MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
				);

				// Alice try to redeem unavailable asset
				assert_noop!(
					MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 20_000 * DOLLARS),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);

				// Alice redeem from DOT pool
				let alice_redeem_amount = 30_000 * DOLLARS;
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_redeem_amount
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - alice_redeem_amount
				);

				// Checking free balance DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount + alice_redeem_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap()
						- TestController::convert_to_wrapped(CurrencyId::DOT, alice_redeem_amount).unwrap()
				);

				// Admin pause redeem operation.
				assert_ok!(TestController::pause_specific_operation(
					admin(),
					CurrencyId::DOT,
					Operation::Redeem
				));

				// Alice try to redeem.
				assert_noop!(
					MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::DOT, 30_000 * DOLLARS),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Checking we have previously values
				// Checking pool available liquidity
				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - alice_redeem_amount
				);

				// Checking free balance DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount + alice_redeem_amount
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap()
						- TestController::convert_to_wrapped(CurrencyId::DOT, alice_redeem_amount).unwrap()
				);
			});
	}

	// Scenario #1 description:
	// The user redeems all assets in the first currency. He has loan in the first currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice deposit 50 ETH;
	// 3. Alice borrow 50 DOT;
	// 4. Alice can't `redeem_underlying` 60 DOT: 10 DOT * 0.9 + 50 ETH * 0.9 in pool < 60 DOT redeem;
	// 5. Alice deposit 10 ETH;
	// 6. Alice `redeem_underlying` 60 DOT;
	// 7. Alice can't `redeem_underlying` 60 ETH.
	#[test]
	fn redeem_underlying_all_assets_with_current_currency_borrowing() {
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
					60_000 * DOLLARS
				));

				// Alice deposit 50 ETH to pool.
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

				// Alice borrow 50 DOT from pool
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
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth).unwrap();
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

				// Alice try to redeem all from DOT pool
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				// Alice redeem all DOTs
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				// Checking free balance DOT/MDOT && ETH/METH for user.
				let expected_amount_redeemed_underlying_assets =
					TestController::convert_from_wrapped(CurrencyId::MDOT, expected_amount_wrapped_tokens_in_dot)
						.unwrap();
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
					+ TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth_secondary)
						.unwrap();
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
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

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

	// Scenario #2 description:
	// The user redeems all assets in the first currency. He has loan in the second currency.
	// Initial exchange rate for all assets equal 1.0;
	// Collateral factor for all assets equal 0.9;
	// 1. Alice deposit 60 DOT;
	// 2. Alice borrow 50 ETH;
	// 3. Alice can't `redeem_underlying` 60 DOT: 50 ETH * 0.9 in pool < 60 DOT redeem;
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

	// Scenario #3 description:
	// The user redeems all assets in the first currency. He has loan in the second currency and
	// deposit in the third currency.
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

				// Alice deposit to BTC pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

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

				// Alice try to redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem_underlying(
						Origin::signed(ALICE),
						CurrencyId::DOT,
						alice_deposited_amount_in_dot
					),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				// Alice redeem all DOTs
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestController::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot)
						.unwrap();
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

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

	// Scenario #4 description:
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

				// Bob deposit to BTC pool
				let bob_deposited_amount_in_btc = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::BTC,
					bob_deposited_amount_in_btc
				));

				// Bob borrow from DOT pool
				let bob_borrowed_amount_in_dot = 5_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// Alice redeem all DOTs.
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestController::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot)
						.unwrap();
				assert_ok!(MinterestProtocol::redeem_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount_in_dot
				));

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					10_000 * DOLLARS + alice_deposited_amount_in_dot
						- alice_redeemed_amount_in_dot
						- bob_borrowed_amount_in_dot
				);

				// Checking free balance DOT && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					bob_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &BOB),
					ONE_HUNDRED - bob_deposited_amount_in_btc
				);
			});
	}

	#[test]
	fn redeem_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, false)
			.pool_initial(CurrencyId::DOT)
			.pool_balance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Alice deposit to DOT pool
				let alice_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_deposited_amount
				));

				// Checking pool available liquidity
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount
				);

				// Checking free balance DOT && MDOT in pool.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount
				);
				let expected_amount_wrapped_tokens =
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens
				);

				// Admin pause redeem operation.
				assert_ok!(TestController::pause_specific_operation(
					admin(),
					CurrencyId::DOT,
					Operation::Redeem
				));

				// Alice try to redeem unavailable asset
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::MDOT),
					MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
				);

				// Alice try to redeem. Redeem is paused
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Admin unpause redeem operation.
				assert_ok!(TestController::unpause_specific_operation(
					admin(),
					CurrencyId::DOT,
					Operation::Redeem
				));

				// Alice redeem from DOT pool
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking pool available liquidity
				let expected_amount_underlying_assets =
					TestController::convert_from_wrapped(CurrencyId::MDOT, expected_amount_wrapped_tokens).unwrap();
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					alice_deposited_amount - expected_amount_underlying_assets
				);

				// Checking free balance DOT && MDOT
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount + expected_amount_underlying_assets
				);
				// Expected 0
				assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 0);

				// Alice try to redeem. MDOT Balance is zero.
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::NumberOfWrappedTokensIsZero
				);
			});
	}

	#[test]
	// Scenario description:
	// FIXME: add description
	fn redeem_scenario_1_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(ALICE, CurrencyId::ETH, ONE_HUNDRED)
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

				// Alice deposit to ETH pool
				let alice_deposited_amount_in_eth = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth
				));

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
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_dot).unwrap();
				assert_eq!(
					Currencies::free_balance(CurrencyId::MDOT, &ALICE),
					expected_amount_wrapped_tokens_in_dot
				);
				let expected_amount_wrapped_tokens_in_eth =
					TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth).unwrap();
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

				// Alice try to redeem all from DOT pool
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Alice add liquidity to ETH pool
				let alice_deposited_amount_in_eth_secondary = 10_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::ETH,
					alice_deposited_amount_in_eth_secondary
				));

				// Alice redeem all DOTs
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking free balance DOT/MDOT && ETH/METH in pool.
				let expected_amount_redeemed_underlying_assets =
					TestController::convert_from_wrapped(CurrencyId::MDOT, expected_amount_wrapped_tokens_in_dot)
						.unwrap();
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
					+ TestController::convert_to_wrapped(CurrencyId::DOT, alice_deposited_amount_in_eth_secondary)
						.unwrap();
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
				assert_eq!(
					TestPools::pools(CurrencyId::DOT).total_borrowed,
					alice_borrowed_amount_in_dot
				);

				// Alice try to redeem all from ETH pool
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::ETH),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	#[test]
	// Scenario description:
	// FIXME: add description
	fn redeem_scenario_2_should_work() {
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn redeem_scenario_3_should_work() {
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

				// Alice deposit to DOT pool
				let alice_deposited_amount_in_btc = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc
				));

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

				// Alice try to redeem all DOTs
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);

				// Alice add liquidity to BTC pool
				let alice_deposited_amount_in_btc_secondary = 40_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(ALICE),
					CurrencyId::BTC,
					alice_deposited_amount_in_btc_secondary
				));

				// Alice redeem all DOTs
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestController::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot)
						.unwrap();
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

				// Alice try to redeem all BTC.
				assert_noop!(
					MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::BTC),
					MinterestProtocolError::<Test>::RedeemControllerRejection
				);
			});
	}

	#[test]
	// Scenario description:
	// FIXME: add description
	fn redeem_scenario_4_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.user_balance(BOB, CurrencyId::BTC, ONE_HUNDRED)
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

				// Bob deposit to BTC pool
				let bob_deposited_amount_in_btc = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::BTC,
					bob_deposited_amount_in_btc
				));

				// Bob borrow from DOT pool
				let bob_borrowed_amount_in_dot = 5_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_borrowed_amount_in_dot
				));

				// Alice redeem all DOTs.
				let alice_current_balance_amount_in_m_dot = Currencies::free_balance(CurrencyId::MDOT, &ALICE);
				let alice_redeemed_amount_in_dot =
					TestController::convert_from_wrapped(CurrencyId::MDOT, alice_current_balance_amount_in_m_dot)
						.unwrap();
				assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking pool available liquidity.
				assert_eq!(
					TestPools::get_pool_available_liquidity(CurrencyId::DOT),
					10_000 * DOLLARS + alice_deposited_amount_in_dot
						- alice_redeemed_amount_in_dot
						- bob_borrowed_amount_in_dot
				);

				// Checking free balance DOT && BTC for user.
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &ALICE),
					ONE_HUNDRED - alice_deposited_amount_in_dot + alice_redeemed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::DOT, &BOB),
					bob_borrowed_amount_in_dot
				);
				assert_eq!(
					Currencies::free_balance(CurrencyId::BTC, &BOB),
					ONE_HUNDRED - bob_deposited_amount_in_btc
				);
			});
	}

	#[test]
	// FIXME: set environment
	fn borrow_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				60_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);

			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 100_000 * DOLLARS),
				MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
			);
			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::MDOT, 60_000 * DOLLARS),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);

			assert_ok!(MinterestProtocol::borrow(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				30_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				30_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30_000 * DOLLARS);
			assert_eq!(
				TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				30_000 * DOLLARS
			);

			// pool_available_liquidity (DOT) = 30
			// Admin depositing to the insurance 10 DOT, now pool_available_liquidity = 30 + 10 = 40 DOT
			assert_ok!(TestController::deposit_insurance(
				Origin::signed(ADMIN),
				CurrencyId::DOT,
				10_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				40_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), 90_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), 0);
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 10_000 * DOLLARS);

			// Bob can't borrow 35 DOT.
			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(BOB), CurrencyId::DOT, 35_000 * DOLLARS),
				MinterestProtocolError::<Test>::BorrowControllerRejection
			);
		});
	}

	#[test]
	// Scenario description:
	// FIXME: add description
	fn borrow_scenario_1_should_work() {
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn borrow_scenario_2_should_work() {
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn borrow_scenario_3_should_work() {
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn borrow_scenario_4_should_work() {
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

				// Alice try to borrow from ETH pool
				let alice_borrowed_amount = 40_000 * DOLLARS;
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn set_insurance_factor_scenario_1_should_work() {
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

				// Alice try to borrow from DOT pool
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

				// Jump to 10 block number.
				System::set_block_number(10);

				// Set insurance factor equal to zero.
				assert_ok!(TestController::set_insurance_factor(admin(), CurrencyId::DOT, 0, 1));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				// Checking pool total insurance.
				assert_eq!(TestPools::pools(CurrencyId::DOT).total_insurance, BALANCE_ZERO);
			});
	}

	#[test]
	// Scenario description:
	// FIXME: add description
	fn set_insurance_factor_scenario_2_should_work() {
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

				// Alice try to borrow from DOT pool
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

				// Jump to 10 block number.
				System::set_block_number(10);

				// Set insurance factor equal to zero.
				assert_ok!(TestController::set_insurance_factor(admin(), CurrencyId::DOT, 1, 2));

				// Alice repay full loan in DOTs.
				assert_ok!(MinterestProtocol::repay_all(Origin::signed(ALICE), CurrencyId::DOT));

				let expected_interest_accumulated: Balance = 810_000_000_000_000;

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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn calculate_borrow_interest_rate_scenario_1_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, BALANCE_ZERO)
			.build()
			.execute_with(|| {
				// Calculate expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = calculate_borrow_interest_rate_mock(CurrencyId::DOT).unwrap();

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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn calculate_borrow_interest_rate_scenario_2_should_work() {
		ExtBuilder::default()
			.user_balance(ALICE, CurrencyId::DOT, ONE_HUNDRED)
			.pool_user_data(ALICE, CurrencyId::DOT, BALANCE_ZERO, RATE_ZERO, true)
			.pool_total_insurance(CurrencyId::DOT, ONE_HUNDRED)
			.build()
			.execute_with(|| {
				// Calculate expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = calculate_borrow_interest_rate_mock(CurrencyId::DOT).unwrap();

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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn calculate_borrow_interest_rate_scenario_3_should_work() {
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

				// Calculate expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = calculate_borrow_interest_rate_mock(CurrencyId::DOT).unwrap();

				// Alice try to borrow from DOT pool
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn calculate_borrow_interest_rate_scenario_4_should_work() {
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

				// Set next block number
				System::set_block_number(1);

				// Calculate expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = calculate_borrow_interest_rate_mock(CurrencyId::DOT).unwrap();

				// Alice try to borrow from DOT pool
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

	#[test]
	// Scenario description:
	// FIXME: add description
	fn calculate_borrow_interest_rate_scenario_5_should_work() {
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

				// Set next block number
				System::set_block_number(1);

				// Alice borrow from DOT pool
				let alice_borrowed_amount_in_dot = 20_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(ALICE),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				// Set next block number
				System::set_block_number(2);

				// Bob deposit to DOT pool
				let bob_deposited_amount = 60_000 * DOLLARS;
				assert_ok!(MinterestProtocol::deposit_underlying(
					Origin::signed(BOB),
					CurrencyId::DOT,
					bob_deposited_amount
				));

				// Set next block number
				System::set_block_number(3);

				// Calculate expected borrow interest rate based on params before fn accrue_interest_rate called
				let expected_borrow_rate_mock = calculate_borrow_interest_rate_mock(CurrencyId::DOT).unwrap();

				// Alice try to borrow from DOT pool
				let bob_borrowed_amount_in_dot = 50_000 * DOLLARS;
				assert_ok!(MinterestProtocol::borrow(
					Origin::signed(BOB),
					CurrencyId::DOT,
					alice_borrowed_amount_in_dot
				));

				assert_eq!(
					TestController::controller_dates(CurrencyId::DOT).borrow_rate,
					expected_borrow_rate_mock
				);
			});
	}

	#[test]
	// FIXME: set environment
	fn repay_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				60_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);

			assert_ok!(MinterestProtocol::borrow(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				30_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				30_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30_000 * DOLLARS);
			assert_eq!(
				TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				30_000 * DOLLARS
			);

			assert_noop!(
				MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::MDOT, 10_000 * DOLLARS),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);
			assert_noop!(
				MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 100_000 * DOLLARS),
				MinterestProtocolError::<Test>::NotEnoughUnderlyingsAssets
			);

			assert_ok!(MinterestProtocol::repay(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				20_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				50_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 50_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 10_000 * DOLLARS);
			assert_eq!(
				TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				10_000 * DOLLARS
			);
		});
	}

	#[test]
	// FIXME: set environment
	fn repay_on_behalf_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				60_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 100_000 * DOLLARS);

			assert_ok!(MinterestProtocol::borrow(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				30_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				30_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30_000 * DOLLARS);
			assert_eq!(
				TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				30_000 * DOLLARS
			);

			assert_noop!(
				MinterestProtocol::repay_on_behalf(Origin::signed(BOB), CurrencyId::MDOT, ALICE, 10_000 * DOLLARS),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);
			assert_noop!(
				MinterestProtocol::repay_on_behalf(Origin::signed(BOB), CurrencyId::DOT, ALICE, 120_000 * DOLLARS),
				MinterestProtocolError::<Test>::NotEnoughUnderlyingsAssets
			);
			assert_noop!(
				MinterestProtocol::repay_on_behalf(Origin::signed(BOB), CurrencyId::DOT, BOB, 100_000 * DOLLARS),
				//FIXME: is it Ok to check internal error?
				MinterestProtocolError::<Test>::RepayAmountToBig
			);

			assert_ok!(MinterestProtocol::repay_on_behalf(
				Origin::signed(BOB),
				CurrencyId::DOT,
				ALICE,
				20_000 * DOLLARS
			));
			assert_eq!(
				TestPools::get_pool_available_liquidity(CurrencyId::DOT),
				50_000 * DOLLARS
			);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60_000 * DOLLARS);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 80_000 * DOLLARS);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 10_000 * DOLLARS);
			assert_eq!(
				TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT),
				10_000 * DOLLARS
			);
		});
	}
}
