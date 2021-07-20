//! # Integration Tests Pallet
//!
//! ## Overview
//!
//! Integration Tests pallet is responsible for checking complex test cases with several pallets
//! involved.
//! Tests are split into different files depending on what pallet they are related to. There is also
//! a scenario_tests.rs file which isn`t related to any particular pallet.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests {
	use controller::{ControllerData, PauseKeeper};
	use frame_support::{
		assert_noop, assert_ok, ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, PalletId,
	};
	use frame_system::{offchain::SendTransactionTypes, EnsureSignedBy};
	use liquidity_pools::{PoolData, PoolUserData};
	use minterest_model::MinterestModelData;
	pub use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
	use minterest_primitives::{Balance, CurrencyId, Price, Rate};
	use minterest_protocol::{Error as MinterestProtocolError, PoolInitData};
	use orml_traits::{parameter_type_with_key, MultiCurrency};
	use pallet_traits::{CurrencyConverter, PoolsManager, PricesManager};
	use sp_core::H256;
	use sp_runtime::{
		testing::{Header, TestXt},
		traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, One, Zero},
		transaction_validity::TransactionPriority,
		FixedPointNumber,
	};
	use sp_std::cell::RefCell;
	use std::collections::HashMap;
	use test_helper::*;

	mod controller_tests;
	mod liquidity_pools_tests;
	mod minterest_model_tests;
	mod minterest_protocol_tests;
	mod mnt_token_tests;
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
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
			Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>, Config<T>},
			Currencies: orml_currencies::{Pallet, Call, Event<T>},
			MinterestProtocol: minterest_protocol::{Pallet, Storage, Call, Event<T>},
			TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
			TestLiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>},
			TestController: controller::{Pallet, Storage, Call, Event, Config<T>},
			TestMinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
			TestDex: dex::{Pallet, Storage, Call, Event<T>},
			TestMntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
			TestRiskManager: risk_manager::{Pallet, Storage, Call, Event<T>, Config<T>},
			TestWhitelist: whitelist_module::{Pallet, Storage, Call, Event<T>, Config<T>},
		}
	);

	ord_parameter_types! {
		pub const ZeroAdmin: AccountId = 0;
	}

	parameter_types! {
		pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdy/min");
		pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"lqdn/min");
		pub const MntTokenPalletId: PalletId = PalletId(*b"min/mntt");
		pub LiquidityPoolAccountId: AccountId = LiquidityPoolsPalletId::get().into_account();
		pub LiquidationPoolAccountId: AccountId = LiquidationPoolsPalletId::get().into_account();
		pub MntTokenAccountId: AccountId = MntTokenPalletId::get().into_account();
		pub InitialExchangeRate: Rate = Rate::one();
		pub EnabledUnderlyingAssetsIds: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset);
		pub EnabledWrappedTokensId: Vec<CurrencyId> = CurrencyId::get_enabled_tokens_in_protocol(WrappedToken);
	}

	mock_impl_system_config!(Test);
	mock_impl_balances_config!(Test);
	mock_impl_orml_tokens_config!(Test);
	mock_impl_orml_currencies_config!(Test);
	mock_impl_liquidity_pools_config!(Test);
	mock_impl_liquidation_pools_config!(Test);
	mock_impl_controller_config!(Test, ZeroAdmin);
	mock_impl_minterest_model_config!(Test, ZeroAdmin);
	mock_impl_dex_config!(Test);
	mock_impl_minterest_protocol_config!(Test, ZeroAdmin);
	mock_impl_mnt_token_config!(Test, ZeroAdmin);
	mock_impl_risk_manager_config!(Test, ZeroAdmin);
	mock_impl_whitelist_module_config!(Test, ZeroAdmin);

	thread_local! {
		static UNDERLYING_PRICE: RefCell<HashMap<CurrencyId, Price>> = RefCell::new(
			[
				(DOT, Price::one()),
				(ETH, Price::one()),
				(BTC, Price::one()),
				(KSM, Price::one()),
			]
			.iter()
			.cloned()
			.collect());
	}

	pub struct MockPriceSource;
	impl MockPriceSource {
		pub fn set_underlying_price(currency_id: CurrencyId, price: Price) {
			UNDERLYING_PRICE.with(|v| v.borrow_mut().insert(currency_id, price));
		}
	}

	impl PricesManager<CurrencyId> for MockPriceSource {
		fn get_underlying_price(currency_id: CurrencyId) -> Option<Price> {
			UNDERLYING_PRICE.with(|v| v.borrow().get(&currency_id).copied())
		}

		fn lock_price(_currency_id: CurrencyId) {}

		fn unlock_price(_currency_id: CurrencyId) {}
	}

	thread_local! {
		static FOUR: RefCell<Vec<u64>> = RefCell::new(vec![4]);
	}

	pub struct ExtBuilder {
		endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
		pools: Vec<(CurrencyId, PoolData)>,
		pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
		minted_pools: Vec<(CurrencyId, Balance)>,
		controller_data: Vec<(CurrencyId, ControllerData<BlockNumber>)>,
		minterest_model_params: Vec<(CurrencyId, MinterestModelData)>,
		mnt_claim_threshold: Balance,
		liquidation_fee: Vec<(CurrencyId, Rate)>,
		liquidation_threshold: Rate,
	}

	impl Default for ExtBuilder {
		fn default() -> Self {
			Self {
				endowed_accounts: vec![],
				pools: vec![],
				pool_user_data: vec![],
				minted_pools: vec![],
				controller_data: vec![
					(
						DOT,
						ControllerData {
							last_interest_accrued_block: 0,
							protocol_interest_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
							protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						},
					),
					(
						ETH,
						ControllerData {
							last_interest_accrued_block: 0,
							protocol_interest_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
							protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						},
					),
					(
						BTC,
						ControllerData {
							last_interest_accrued_block: 0,
							protocol_interest_factor: Rate::saturating_from_rational(1, 10),
							max_borrow_rate: Rate::saturating_from_rational(5, 1000),
							collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
							borrow_cap: None,
							protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
						},
					),
				],
				minterest_model_params: vec![
					(
						DOT,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
					(
						ETH,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
					(
						BTC,
						MinterestModelData {
							kink: Rate::saturating_from_rational(8, 10),
							base_rate_per_block: Rate::zero(),
							multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
							jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
						},
					),
				],
				mnt_claim_threshold: 0, // disable by default
				liquidation_fee: vec![
					(DOT, Rate::saturating_from_rational(5, 100)),
					(ETH, Rate::saturating_from_rational(5, 100)),
					(BTC, Rate::saturating_from_rational(5, 100)),
					(KSM, Rate::saturating_from_rational(5, 100)),
				],
				liquidation_threshold: Rate::saturating_from_rational(3, 100),
			}
		}
	}

	impl ExtBuilder {
		pub fn set_risk_manager_params(
			mut self,
			liquidation_fee: Vec<(CurrencyId, Rate)>,
			liquidation_threshold: Rate,
		) -> Self {
			self.liquidation_fee = liquidation_fee;
			self.liquidation_threshold = liquidation_threshold;
			self
		}

		pub fn set_controller_data(mut self, pools: Vec<(CurrencyId, ControllerData<BlockNumber>)>) -> Self {
			self.controller_data = pools;
			self
		}

		pub fn set_minterest_model_params(mut self, pools: Vec<(CurrencyId, MinterestModelData)>) -> Self {
			self.minterest_model_params = pools;
			self
		}

		pub fn mnt_enabled_pools(mut self, pools: Vec<(CurrencyId, Balance)>) -> Self {
			self.minted_pools = pools;
			self
		}

		pub fn user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
			self.endowed_accounts.push((user, currency_id, balance));
			self
		}

		pub fn pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
			self.endowed_accounts
				.push((TestPools::pools_account_id(), currency_id, balance));
			self
		}

		pub fn pool_borrow_underlying(mut self, pool_id: CurrencyId, borrowed: Balance) -> Self {
			self.pools.push((
				pool_id,
				PoolData {
					borrowed,
					borrow_index: Rate::one(),
					protocol_interest: Balance::zero(),
				},
			));
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

		pub fn pool_initial(mut self, pool_id: CurrencyId) -> Self {
			self.pools.push((
				pool_id,
				PoolData {
					borrowed: Balance::zero(),
					borrow_index: Rate::one(),
					protocol_interest: Balance::zero(),
				},
			));
			self
		}

		pub fn mnt_account_balance(mut self, balance: Balance) -> Self {
			self.endowed_accounts
				.push((TestMntToken::get_account_id(), MNT, balance));
			self
		}

		pub fn mnt_claim_threshold(mut self, threshold: Balance) -> Self {
			self.mnt_claim_threshold = threshold;
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

			controller::GenesisConfig::<Test> {
				controller_params: self.controller_data,
				pause_keepers: vec![
					(ETH, PauseKeeper::all_unpaused()),
					(DOT, PauseKeeper::all_unpaused()),
					(KSM, PauseKeeper::all_paused()),
					(BTC, PauseKeeper::all_unpaused()),
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

			minterest_model::GenesisConfig::<Test> {
				minterest_model_params: self.minterest_model_params,
				_phantom: Default::default(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			risk_manager::GenesisConfig::<Test> {
				liquidation_fee: self.liquidation_fee,
				liquidation_threshold: self.liquidation_threshold,
				_phantom: Default::default(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			mnt_token::GenesisConfig::<Test> {
				mnt_claim_threshold: self.mnt_claim_threshold,
				minted_pools: self.minted_pools,
				_phantom: Default::default(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));
			ext
		}
	}
}
