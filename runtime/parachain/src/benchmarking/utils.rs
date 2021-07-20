#![allow(unused_imports)]

use crate::{
	AccountId, Balance, Currencies, CurrencyId, LiquidityPools, MinterestProtocol, MntTokenPalletId, Origin, Rate,
	Runtime, Vec, Whitelist, BTC, DOLLARS, DOT, ETH, KSM, MNT,
};

use frame_benchmarking::account;
use frame_support::pallet_prelude::DispatchResultWithPostInfo;
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use liquidity_pools::Pool;
use orml_traits::MultiCurrency;
use pallet_traits::LiquidityPoolStorageProvider;
use sp_runtime::{
	traits::{AccountIdConversion, One, StaticLookup, Zero},
	FixedPointNumber,
};

pub const SEED: u32 = 0;

pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Config>::Lookup::unlookup(who)
}

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) -> DispatchResultWithPostInfo {
	<Currencies as MultiCurrency<_>>::deposit(currency_id, &who, balance)?;
	Ok(().into())
}

pub fn enable_is_collateral_mock<T: frame_system::Config<Origin = Origin>>(
	origin: OriginFor<T>,
	currency_id: CurrencyId,
) -> DispatchResultWithPostInfo {
	MinterestProtocol::enable_is_collateral(origin.into(), currency_id)?;
	Ok(().into())
}

pub fn enable_whitelist_mode_and_add_member(who: &AccountId) -> DispatchResultWithPostInfo {
	Whitelist::switch_whitelist_mode(RawOrigin::Root.into(), true)?;
	Whitelist::add_member(RawOrigin::Root.into(), who.clone())?;
	Ok(().into())
}

pub(crate) fn create_pools(pools: &Vec<CurrencyId>) {
	pools.into_iter().for_each(|pool_id| {
		LiquidityPools::set_pool_data(
			*pool_id,
			Pool {
				borrowed: Balance::zero(),
				borrow_index: Rate::one(),
				protocol_interest: Balance::zero(),
			},
		);
	});
}

pub(crate) fn prepare_for_mnt_distribution(pools: Vec<CurrencyId>) -> Result<(), &'static str> {
	let helper: AccountId = account("helper", 0, SEED);
	enable_whitelist_mode_and_add_member(&helper)?;
	set_balance(MNT, &MntTokenPalletId::get().into_account(), 1_000_000 * DOLLARS)?;
	pools.into_iter().try_for_each(|pool_id| -> Result<(), &'static str> {
		set_balance(pool_id, &helper, 50_000 * DOLLARS)?;
		MinterestProtocol::deposit_underlying(RawOrigin::Signed(helper.clone()).into(), pool_id, 50_000 * DOLLARS)?;
		MinterestProtocol::enable_is_collateral(Origin::signed(helper.clone()).into(), pool_id)?;
		MinterestProtocol::borrow(RawOrigin::Signed(helper.clone()).into(), pool_id, 10_000 * DOLLARS)?;
		Ok(())
	})
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use controller::{ControllerData, PauseKeeper};
	use frame_support::traits::GenesisBuild;
	use liquidity_pools::Pool;
	use minterest_model::MinterestModelData;
	use minterest_primitives::{
		constants::{currency::DOLLARS, PROTOCOL_INTEREST_TRANSFER_THRESHOLD},
		{Balance, Rate},
	};
	use sp_runtime::{traits::Zero, FixedU128};

	// This GenesisConfig is a copy of testnet_genesis.
	pub fn test_externalities() -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		liquidity_pools::GenesisConfig::<Runtime> {
			pools: vec![
				(
					ETH,
					Pool {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					DOT,
					Pool {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					KSM,
					Pool {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					Pool {
						borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						protocol_interest: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		controller::GenesisConfig::<Runtime> {
			controller_params: vec![
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
					KSM,
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
			pause_keepers: vec![
				(ETH, PauseKeeper::all_unpaused()),
				(DOT, PauseKeeper::all_unpaused()),
				(KSM, PauseKeeper::all_unpaused()),
				(BTC, PauseKeeper::all_unpaused()),
			],
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		minterest_model::GenesisConfig::<Runtime> {
			minterest_model_params: vec![
				(
					ETH,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					DOT,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					KSM,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
				(
					BTC,
					MinterestModelData {
						kink: Rate::saturating_from_rational(8, 10), // 0.8 = 80 %
						base_rate_per_block: Rate::zero(),
						multiplier_per_block: Rate::saturating_from_rational(9, 1_000_000_000), // 0.047304 PerYear
						jump_multiplier_per_block: Rate::saturating_from_rational(207, 1_000_000_000), // 1.09 PerYear
					},
				),
			],
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		risk_manager::GenesisConfig::<Runtime> {
			liquidation_fee: vec![
				(DOT, FixedU128::saturating_from_rational(5, 100)), // 5%
				(ETH, FixedU128::saturating_from_rational(5, 100)), // 5%
				(BTC, FixedU128::saturating_from_rational(5, 100)), // 5%
				(KSM, FixedU128::saturating_from_rational(5, 100)), // 5%
			],
			liquidation_threshold: FixedU128::saturating_from_rational(3, 100), // 3%
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		module_prices::GenesisConfig::<Runtime> {
			locked_price: vec![
				(DOT, FixedU128::saturating_from_integer(2)),
				(KSM, FixedU128::saturating_from_integer(2)),
				(ETH, FixedU128::saturating_from_integer(2)),
				(BTC, FixedU128::saturating_from_integer(2)),
			],
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		mnt_token::GenesisConfig::<Runtime> {
			mnt_claim_threshold: 0, // disable by default
			minted_pools: vec![
				(DOT, 2 * DOLLARS),
				(ETH, 2 * DOLLARS),
				(KSM, 2 * DOLLARS),
				(BTC, 2 * DOLLARS),
			],
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		storage.into()
	}
}
