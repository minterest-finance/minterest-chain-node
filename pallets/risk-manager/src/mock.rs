/// Mocks for the RiskManager pallet.
use super::*;
use crate as risk_manager;
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
	pools: Vec<(CurrencyId, Pool)>,
	pool_user_data: Vec<(CurrencyId, AccountId, PoolUserData)>,
	liquidation_fee: Vec<(CurrencyId, Rate)>,
}

impl ExternalityBuilder {
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

	pub fn set_liquidation_fees(mut self, liquidation_fees: Vec<(CurrencyId, Rate)>) -> Self {
		// self.liquidation_fee.extend_from_slice(&liquidation_fees);
		liquidation_fees
			.into_iter()
			.for_each(|(pool_id, liquidation_fee)| self.liquidation_fee.push((pool_id, liquidation_fee)));
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		liquidity_pools::GenesisConfig::<Test> {
			pools: self.pools,
			pool_user_data: self.pool_user_data,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		risk_manager::GenesisConfig::<Test> {
			liquidation_fee: vec![],
			liquidation_threshold: Default::default(),
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
