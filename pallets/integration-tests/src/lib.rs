#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use frame_support::{assert_noop, assert_ok, impl_outer_origin, parameter_types};
	use frame_system::{self as system};
	use liquidity_pools::Reserve;
	use minterest_primitives::{Balance, CurrencyId, Rate};
	use orml_currencies::Currency;
	use pallet_traits::Borrowing;
	use sp_core::H256;
	use sp_runtime::DispatchResult;
	use sp_runtime::{
		testing::Header,
		traits::{IdentityLookup, Zero},
		ModuleId, Perbill,
	};

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
			_underlying_asset_id: CurrencyId,
			_amount_borrowed: Balance,
			_who: &AccountId,
		) -> DispatchResult {
			Ok(())
		}

		fn update_state_on_repay(
			_underlying_asset_id: CurrencyId,
			_amount_borrowed: Balance,
			_who: &AccountId,
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
		type UnderlyingAssetId = UnderlyingAssetId;
		type Borrowing = MockBorrowing;
	}

	parameter_types! {
		pub const InitialExchangeRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
		pub const MaxBorrowRate: Rate = Rate::from_inner(1_000_000_000_000_000_000);
		pub const InsuranceFactor: Rate = Rate::from_inner(1_000_000_000_000_000_000);
	}

	impl controller::Trait for Test {
		type Event = ();
		type InitialExchangeRate = InitialExchangeRate;
		type InsuranceFactor = InsuranceFactor;
		type MaxBorrowRate = MaxBorrowRate;
	}

	pub const ALICE: AccountId = 1;
	pub const BOB: AccountId = 2;
	pub const ONE_MILL: Balance = 1_000_000;
	pub const ONE_HUNDRED: Balance = 100;
	pub type MinterestProtocol = minterest_protocol::Module<Test>;
	pub type MTokens = m_tokens::Module<Test>;
	pub type Pools = liquidity_pools::Module<Test>;

	pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: vec![
				(ALICE, CurrencyId::MINT, ONE_MILL),
				(ALICE, CurrencyId::DOT, ONE_HUNDRED),
				(BOB, CurrencyId::MINT, ONE_MILL),
				(BOB, CurrencyId::DOT, ONE_HUNDRED),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			reserves: vec![
				(
					CurrencyId::ETH,
					Reserve {
						total_balance: Balance::zero(),
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						current_exchange_rate: Rate::from_inner(1),
						is_lock: true,
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::DOT,
					Reserve {
						total_balance: Balance::zero(),
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						current_exchange_rate: Rate::from_inner(1),
						is_lock: true,
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::KSM,
					Reserve {
						total_balance: Balance::zero(),
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						current_exchange_rate: Rate::from_inner(1),
						is_lock: true,
						total_insurance: Balance::zero(),
					},
				),
				(
					CurrencyId::BTC,
					Reserve {
						total_balance: Balance::zero(),
						current_interest_rate: Rate::from_inner(0),
						total_borrowed: Balance::zero(),
						current_exchange_rate: Rate::from_inner(1),
						is_lock: true,
						total_insurance: Balance::zero(),
					},
				),
			],
			reserve_user_data: vec![],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
	/* ----------------------------------------------------------------------------------------- */

	// MinterestProtocol tests
	#[test]
	fn deposit_underlying_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
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
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

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
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 90);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 10);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 90);
		});
	}

	#[test]
	fn redeem_underlying_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

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
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 30);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 30);
		});
	}

	#[test]
	fn redeem_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

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
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_ok!(MinterestProtocol::redeem_wrapped(
				Origin::signed(ALICE),
				CurrencyId::MDOT,
				35
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 25);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 75);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 25);

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
	fn getting_assets_from_reserve_by_different_users_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::redeem_underlying(Origin::signed(BOB), CurrencyId::DOT, 30),
				MinterestProtocolError::<Test>::NotEnoughWrappedTokens
			);

			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(BOB),
				CurrencyId::DOT,
				7
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 67);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &BOB), 93);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &BOB), 7);
		});
	}

	#[test]
	fn borrow_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 100),
				MinterestProtocolError::<Test>::NotEnoughLiquidityAvailable
			);
			assert_noop!(
				MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::MDOT, 60),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);

			assert_ok!(MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 30));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 30);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);
		});
	}

	#[test]
	fn repay_should_work() {
		new_test_ext().execute_with(|| {
			assert_ok!(Pools::unlock_reserve_transactions(Origin::root(), CurrencyId::DOT));
			assert_ok!(MinterestProtocol::deposit_underlying(
				Origin::signed(ALICE),
				CurrencyId::DOT,
				60
			));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 60);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 40);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_ok!(MinterestProtocol::borrow(Origin::signed(ALICE), CurrencyId::DOT, 30));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 30);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 70);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);

			assert_noop!(
				MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::MDOT, 10),
				MinterestProtocolError::<Test>::NotValidUnderlyingAssetId
			);
			assert_noop!(
				MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 100),
				MinterestProtocolError::<Test>::NotEnoughUnderlyingsAssets
			);

			assert_ok!(MinterestProtocol::repay(Origin::signed(ALICE), CurrencyId::DOT, 20));
			assert_eq!(Pools::get_reserve_available_liquidity(CurrencyId::DOT), 50);
			assert_eq!(MTokens::free_balance(CurrencyId::DOT, &ALICE), 50);
			assert_eq!(MTokens::free_balance(CurrencyId::MDOT, &ALICE), 60);
		});
	}
}
