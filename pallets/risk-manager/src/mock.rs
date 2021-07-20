/// Mocks for the RiskManager pallet.
use super::*;
use crate as risk_manager;
use controller::{ControllerData, PauseKeeper};
use frame_support::{ord_parameter_types, pallet_prelude::GenesisBuild, parameter_types, PalletId};
use frame_system::EnsureSignedBy;
use liquidity_pools::{Pool, PoolUserData};
use minterest_primitives::currency::CurrencyType::{UnderlyingAsset, WrappedToken};
pub use minterest_primitives::{Balance, Price, Rate};
use orml_traits::parameter_type_with_key;
use pallet_traits::PricesManager;
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};
use sp_std::collections::btree_map::BTreeMap;
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
		TestPools: liquidity_pools::{Pallet, Storage, Call, Config<T>},
		TestRiskManager: risk_manager::{Pallet, Storage, Call, Event<T>, Config<T>, ValidateUnsigned},
		TestController: controller::{Pallet, Storage, Call, Event, Config<T>},
		TestMinterestModel: minterest_model::{Pallet, Storage, Call, Event, Config<T>},
		TestMntToken: mnt_token::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestMinterestProtocol: minterest_protocol::{Pallet, Storage, Call, Event<T>},
		TestLiquidationPools: liquidation_pools::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestWhitelist: whitelist_module::{Pallet, Storage, Call, Event<T>, Config<T>},
		TestDex: dex::{Pallet, Storage, Call, Event<T>},
	}
);

ord_parameter_types! {
	pub const ZeroAdmin: AccountId = 0;
}

parameter_types! {
	pub const LiquidityPoolsPalletId: PalletId = PalletId(*b"lqdi/min");
	pub const LiquidationPoolsPalletId: PalletId = PalletId(*b"lqdn/min");
	pub const MntTokenPalletId: PalletId = PalletId(*b"mntt/min");
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
mock_impl_risk_manager_config!(Test, ZeroAdmin);
mock_impl_controller_config!(Test, ZeroAdmin);
mock_impl_minterest_model_config!(Test, ZeroAdmin);
mock_impl_mnt_token_config!(Test, ZeroAdmin);
mock_impl_minterest_protocol_config!(Test, ZeroAdmin);
mock_impl_liquidation_pools_config!(Test);
mock_impl_whitelist_module_config!(Test, ZeroAdmin);
mock_impl_dex_config!(Test);

pub struct MockPriceSource;

impl PricesManager<CurrencyId> for MockPriceSource {
	fn get_underlying_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

#[derive(Default)]
pub struct ExternalityBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	controller_data: Vec<(CurrencyId, ControllerData<BlockNumber>)>,
	liquidation_fee: Vec<(CurrencyId, Rate)>,
	liquidation_threshold: Rate,
}

impl ExternalityBuilder {
	/// Simulates extrinsic `deposit_underlying()` in genesis block.
	///
	///-`who`: the user who performs the operation.
	///-`underlying_asset`: CurrencyId of underlying assets to be transferred into the protocol.
	///-`underlying_amount`: The amount of the asset to be supplied, in units of the underlying
	/// asset.
	pub fn deposit_underlying(self, who: AccountId, underlying_asset: CurrencyId, underlying_amount: Balance) -> Self {
		self.init_pool(underlying_asset, Balance::zero(), Rate::one(), Balance::zero())
			.set_pool_user_data(underlying_asset, who, Balance::zero(), Rate::one(), false)
			.set_controller_data_mock(vec![underlying_asset])
			.set_user_balance(who, underlying_asset.wrapped_asset().unwrap(), underlying_amount)
			.set_pool_balance(underlying_asset, underlying_amount)
	}

	/// Simulates extrinsic `enable_is_collateral()` in genesis block.
	///
	///-`who`: the user who performs the operation.
	///-`pool_id`: CurrencyId of liquidity pool to be enabled as collateral.
	pub fn enable_as_collateral(mut self, who: AccountId, pool_id: CurrencyId) -> Self {
		self.pool_user_data = self
			.pool_user_data
			.into_iter()
			.map(|(p, w, mut pool_user_data)| {
				if p == pool_id && w == who {
					pool_user_data.is_collateral = true
				}
				(p, w, pool_user_data)
			})
			.collect::<Vec<(CurrencyId, AccountId, PoolUserData)>>();
		self
	}

	/// Simulates extrinsic `borrow()` in genesis block.
	///
	///-`who`: the user who performs the operation.
	/// - `underlying_asset`: The currency ID of the underlying asset to be borrowed.
	/// - `underlying_amount`: The amount of the underlying asset to be borrowed.
	///
	/// Note: use only after `deposit_underlying` (does not set up controller params).
	pub fn borrow_underlying(self, who: AccountId, underlying_asset: CurrencyId, borrow_amount: Balance) -> Self {
		self.init_pool(underlying_asset, borrow_amount, Rate::one(), Balance::zero())
			.set_pool_user_data(underlying_asset, who, borrow_amount, Rate::one(), false)
			.set_user_balance(who, underlying_asset, borrow_amount)
			.set_pool_balance(underlying_asset, Balance::zero())
	}

	/// Merges duplicate balances in `endowed_accounts` in a genesis block.
	/// Merges duplicate borrows amount `pool_data` and `pool_user_data` in a genesis block.
	pub fn merge_duplicates(mut self) -> Self {
		self.endowed_accounts = self
			.endowed_accounts
			.iter()
			.fold(
				BTreeMap::<(AccountId, CurrencyId), Balance>::new(),
				|mut acc, (account_id, pool_id, amount)| {
					// merge duplicated accounts
					if let Some(balance) = acc.get_mut(&(*account_id, *pool_id)) {
						*balance = balance
							.checked_add(*amount)
							.expect("balance cannot overflow when building genesis");
					} else {
						acc.insert((account_id.clone(), *pool_id), *amount);
					}
					acc
				},
			)
			.into_iter()
			.map(|((account_id, pool_id), amount)| (account_id, pool_id, amount))
			.collect::<Vec<(AccountId, CurrencyId, Balance)>>();
		self.pool_user_data = self
			.pool_user_data
			.iter()
			.fold(
				BTreeMap::<(CurrencyId, AccountId), PoolUserData>::new(),
				|mut acc, (pool_id, account_id, pool_user_data)| {
					// merge duplicated accounts
					if let Some(user_data) = acc.get_mut(&(*pool_id, *account_id)) {
						user_data.borrowed = user_data
							.borrowed
							.checked_add(pool_user_data.borrowed)
							.expect("balance cannot overflow when building genesis");
					} else {
						acc.insert((*pool_id, account_id.clone()), pool_user_data.clone());
					}
					acc
				},
			)
			.into_iter()
			.map(|((pool_id, account_id), pool_user_data)| (pool_id, account_id, pool_user_data))
			.collect::<Vec<(CurrencyId, AccountId, PoolUserData)>>();
		self.pools = self
			.pools
			.iter()
			.fold(BTreeMap::<CurrencyId, Pool>::new(), |mut acc, (pool_id, pool_data)| {
				// merge duplicated accounts
				if let Some(pool) = acc.get_mut(pool_id) {
					pool.borrowed = pool
						.borrowed
						.checked_add(pool_data.borrowed)
						.expect("balance cannot overflow when building genesis");
				} else {
					acc.insert(*pool_id, pool_data.clone());
				}
				acc
			})
			.into_iter()
			.map(|(pool_id, pool)| (pool_id, pool))
			.collect::<Vec<(CurrencyId, Pool)>>();
		self
	}

	/// Set balance for the particular user.
	/// - 'user': id of users account.
	/// - 'currency_id': currency.
	/// - 'balance': balance value to set.
	pub fn set_user_balance(mut self, user: AccountId, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts.push((user, currency_id, balance));
		self
	}

	/// Set balance for the particular pool.
	/// - 'currency_id': pool id.
	/// - 'balance': balance value to set.
	pub fn set_pool_balance(mut self, currency_id: CurrencyId, balance: Balance) -> Self {
		self.endowed_accounts
			.push((TestPools::pools_account_id(), currency_id, balance));
		self
	}

	/// Set user data for particular pool.
	/// - 'pool_id': pool id.
	/// - 'user': user id.
	/// - 'borrowed': total balance (with accrued interest), after applying the most recent.
	///   balance-changing action.
	/// - 'interest_index': global borrow_index as of the most recent balance-changing action.
	/// - 'is_collateral': can pool be used as collateral for the current user.
	/// - 'liquidation_attempts': number of partial liquidations for debt.
	pub fn set_pool_user_data(
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

	/// Initialize pool.
	/// - 'pool_id': pool currency / id.
	/// - 'borrowed': value of currency borrowed from the pool_id.
	/// - 'borrow_index': index, describing change of borrow interest rate.
	/// - 'protocol_interest': interest of the protocol.
	pub fn init_pool(
		mut self,
		pool_id: CurrencyId,
		borrowed: Balance,
		borrow_index: Rate,
		protocol_interest: Balance,
	) -> Self {
		self.pools.push((
			pool_id,
			Pool {
				borrowed,
				borrow_index,
				protocol_interest,
			},
		));
		self
	}

	pub fn set_liquidation_fees(mut self, liquidation_fees: Vec<(CurrencyId, Rate)>) -> Self {
		self.liquidation_fee.extend_from_slice(&liquidation_fees);
		self
	}

	pub fn set_controller_data_mock(mut self, pools: Vec<CurrencyId>) -> Self {
		pools.into_iter().for_each(|pool_id| {
			self.controller_data.push((
				pool_id,
				ControllerData {
					last_interest_accrued_block: 0,
					protocol_interest_factor: Rate::saturating_from_rational(1, 10),
					max_borrow_rate: Rate::saturating_from_rational(5, 1000),
					collateral_factor: Rate::saturating_from_rational(9, 10), // 90%
					borrow_cap: None,
					protocol_interest_threshold: PROTOCOL_INTEREST_TRANSFER_THRESHOLD,
				},
			))
		});
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			balances: self.endowed_accounts,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		risk_manager::GenesisConfig::<Test> {
			liquidation_fee: self.liquidation_fee,
			liquidation_threshold: self.liquidation_threshold,
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		controller::GenesisConfig::<Test> {
			controller_params: self.controller_data,
			pause_keepers: vec![
				(ETH, PauseKeeper::all_unpaused()),
				(DOT, PauseKeeper::all_unpaused()),
				(KSM, PauseKeeper::all_unpaused()),
				(BTC, PauseKeeper::all_unpaused()),
			],
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
