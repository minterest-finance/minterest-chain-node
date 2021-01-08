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
		ModuleId, Perbill,
	};
	use sp_runtime::{DispatchResult, FixedPointNumber};

	use controller::{ControllerData, PauseKeeper};
	use minterest_protocol::Error as MinterestProtocolError;

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
	pub const ONE_HUNDRED: Balance = 100;
	pub const MAX_MEMBERS: u32 = 16;
	pub type MinterestProtocol = minterest_protocol::Module<Test>;
	pub type TestPools = liquidity_pools::Module<Test>;
	pub type TestController = controller::Module<Test>;
	pub type TestAccounts = accounts::Module<Test>;
	pub type Currencies = orml_currencies::Module<Test>;
	pub type System = frame_system::Module<Test>;

	pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: vec![
				(ALICE, CurrencyId::MINT, ONE_MILL),
				(ALICE, CurrencyId::DOT, ONE_HUNDRED),
				(BOB, CurrencyId::MINT, ONE_MILL),
				(BOB, CurrencyId::DOT, ONE_HUNDRED),
				(ADMIN, CurrencyId::MINT, ONE_MILL),
				(ADMIN, CurrencyId::DOT, ONE_HUNDRED),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: vec![
				(
					CurrencyId::ETH,
					Pool {
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::DOT,
					Pool {
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::KSM,
					Pool {
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::BTC,
					Pool {
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						borrow_index: Rate::saturating_from_rational(1, 1),
						current_exchange_rate: Rate::from_inner(1),
						total_insurance: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![
				(
					ALICE,
					CurrencyId::DOT,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::saturating_from_rational(1, 1),
						collateral: true,
					},
				),
				(
					ALICE,
					CurrencyId::ETH,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::saturating_from_rational(1, 1),
						collateral: true,
					},
				),
				(
					ALICE,
					CurrencyId::KSM,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::saturating_from_rational(1, 1),
						collateral: true,
					},
				),
				(
					ALICE,
					CurrencyId::BTC,
					PoolUserData {
						total_borrowed: 0,
						interest_index: Rate::saturating_from_rational(1, 1),
						collateral: true,
					},
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		controller::GenesisConfig::<Test> {
			controller_dates: vec![
				(
					CurrencyId::ETH,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000),
						jump_multiplier_per_block: Rate::saturating_from_rational(2, 1),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
				(
					CurrencyId::DOT,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000),
						jump_multiplier_per_block: Rate::saturating_from_rational(2, 1),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
				(
					CurrencyId::KSM,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000),
						jump_multiplier_per_block: Rate::saturating_from_rational(2, 1),
						collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					},
				),
				(
					CurrencyId::BTC,
					ControllerData {
						timestamp: 0,
						borrow_rate: Rate::from_inner(0),
						insurance_factor: Rate::saturating_from_rational(1, 10),
						max_borrow_rate: Rate::saturating_from_rational(5, 1000),
						kink: Rate::saturating_from_rational(8, 10),
						base_rate_per_block: Rate::from_inner(0),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000),
						jump_multiplier_per_block: Rate::saturating_from_rational(2, 1),
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
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
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

		accounts::GenesisConfig::<Test> {
			allowed_accounts: vec![(ALICE, ())],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext: sp_io::TestExternalities = t.into();
		ext.execute_with(|| System::set_block_number(1));
		ext.into()
	}
	/* ----------------------------------------------------------------------------------------- */

	// MinterestProtocol tests
	#[test]
	fn deposit_underlying_should_work() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::ETH, 10),
				MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
			);
			assert_noop!(
				MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 10),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);

			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::DOT, 50),
				MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
			);
			assert_noop!(
				MinterestProtocol::deposit_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 100),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);

			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				30
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 90);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 10);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 90);
		});
	}

	#[test]
	fn redeem_underlying_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::DOT, 100),
				MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
			);

			assert_noop!(
				MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 20),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);

			assert_ok!(MinterestProtocol::redeem_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				30
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 30);
		});
	}

	#[test]
	fn redeem_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_ok!(MinterestProtocol::redeem(Origin::signed(ALICE), CurrencyId::DOT));

			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_noop!(
				MinterestProtocol::redeem_underlying(Origin::signed(BOB), CurrencyId::DOT, 30),
				MinterestProtocolError::<Test>::NotEnoughWrappedTokens
			);

			assert_noop!(
				MinterestProtocol::redeem_underlying(Origin::signed(ALICE), CurrencyId::MDOT, 20),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);
		});
	}

	#[test]
	fn redeem_wrapped_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_ok!(MinterestProtocol::redeem_wrapped(
				Origin::signed(ALICE),
				CurrencyId::MDOT,
				35
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 25);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 75);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 25);

			assert_noop!(
				MinterestProtocol::redeem_wrapped(Origin::signed(ALICE), CurrencyId::MDOT, 60),
				MinterestProtocolError::<Test>::NotEnoughWrappedTokens
			);
			assert_noop!(
				MinterestProtocol::redeem_wrapped(Origin::signed(ALICE), CurrencyId::DOT, 20),
				MinterestProtocolError::<Test>::NotValidWrappedTokenId
			);
		});
	}

	#[test]
	fn getting_assets_from_pool_by_different_users_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::redeem_underlying(Origin::signed(BOB), CurrencyId::DOT, 30),
				MinterestProtocolError::<Test>::NotEnoughWrappedTokens
			);

			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(BOB),
				CurrencyId::DOT,
				7
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 67);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &BOB), 93);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &BOB), 7);
		});
	}

	#[test]
	fn borrow_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 100),
				MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
			);
			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::MDOT, 60),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);

			assert_ok!(MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 30));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);

			// pool_available_liquidity (DOT) = 30
			// Admin depositing to the insurance 10 DOT, now pool_available_liquidity = 30 + 10 = 40 DOT
			assert_ok!(TestAccounts::add_member(Origin::root(), ADMIN));
			assert_ok!(TestController::deposit_insurance(
				Origin::signed(ADMIN),
				CurrencyId::DOT,
				10
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ADMIN), 90);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ADMIN), 0);
			assert_eq!(TestPools::get_pool_total_insurance(CurrencyId::DOT), 10);

			// Bob can't borrow 35 DOT.
			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(BOB), CurrencyId::DOT, 35),
				MinterestProtocolError::<Test>::BorrowControllerRejection
			);
		});
	}

	#[test]
	fn repay_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_ok!(MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 30));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 30);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 30);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 30);

			assert_noop!(
				MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::MDOT, 10),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);
			assert_noop!(
				MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 100),
				MinterestProtocolError::<Test>::NotEnoughUnderlyingsAssets
			);

			assert_ok!(MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 20));
			assert_eq!(TestPools::get_pool_available_liquidity(CurrencyId::DOT), 50);
			assert_eq!(Currencies::free_balance(CurrencyId::DOT, &ALICE), 50);
			assert_eq!(Currencies::free_balance(CurrencyId::MDOT, &ALICE), 60);
			assert_eq!(TestPools::get_pool_total_borrowed(CurrencyId::DOT), 10);
			assert_eq!(TestPools::get_user_total_borrowed(&ALICE, CurrencyId::DOT), 10);
		});
	}
}
