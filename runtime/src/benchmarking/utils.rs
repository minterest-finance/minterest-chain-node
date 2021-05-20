use crate::{
	AccountId, Balance, Currencies, CurrencyId, MinterestProtocol, MntTokenModuleId, Origin, Rate, Runtime, Vec,
	WhitelistCouncilMembership, BTC, DOLLARS, DOT, ETH, KSM, MNT,
};

use frame_benchmarking::account;
use frame_support::pallet_prelude::DispatchResultWithPostInfo;
use frame_system::pallet_prelude::OriginFor;
use frame_system::RawOrigin;
use liquidity_pools::Pool;
use orml_traits::MultiCurrency;
use sp_runtime::traits::{AccountIdConversion, StaticLookup};
use sp_runtime::FixedPointNumber;

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
	controller::WhitelistMode::<Runtime>::put(true);
	WhitelistCouncilMembership::add_member(RawOrigin::Root.into(), who.clone())?;
	Ok(().into())
}

pub(crate) fn create_pools(pools: &Vec<CurrencyId>) {
	pools.into_iter().for_each(|pool_id| {
		liquidity_pools::Pools::<Runtime>::insert(
			pool_id,
			Pool {
				total_borrowed: 0,
				borrow_index: Rate::one(),
				total_protocol_interest: 0,
			},
		);
	});
}

pub(crate) fn prepare_for_mnt_distribution(pools: Vec<CurrencyId>) -> Result<(), &'static str> {
	let helper: AccountId = account("helper", 0, SEED);
	enable_whitelist_mode_and_add_member(&helper)?;
	set_balance(MNT, &MntTokenModuleId::get().into_account(), 1_000_000 * DOLLARS)?;
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
	use crate::constants::currency::DOLLARS;
	use crate::constants::PROTOCOL_INTEREST_TRANSFER_THRESHOLD;
	use controller::{ControllerData, PauseKeeper};
	use frame_support::traits::GenesisBuild;
	use liquidity_pools::Pool;
	use minterest_model::MinterestModelData;
	use minterest_primitives::{Balance, Rate};
	use risk_manager::RiskManagerData;
	use sp_runtime::traits::Zero;
	use sp_runtime::FixedU128;

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
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					DOT,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					KSM,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
				(
					BTC,
					Pool {
						total_borrowed: Balance::zero(),
						borrow_index: Rate::one(),
						total_protocol_interest: Balance::zero(),
					},
				),
			],
			pool_user_data: vec![],
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		controller::GenesisConfig::<Runtime> {
			controller_dates: vec![
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
				(
					ETH,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					DOT,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					KSM,
					PauseKeeper {
						deposit_paused: false,
						redeem_paused: false,
						borrow_paused: false,
						repay_paused: false,
						transfer_paused: false,
					},
				),
				(
					BTC,
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
		.assimilate_storage(&mut storage)
		.unwrap();

		minterest_model::GenesisConfig {
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
		}
		.assimilate_storage::<Runtime>(&mut storage)
		.unwrap();

		risk_manager::GenesisConfig {
			risk_manager_dates: vec![
				(
					ETH,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 200_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					DOT,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 100_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					KSM,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 200_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
				(
					BTC,
					RiskManagerData {
						max_attempts: 2,
						min_partial_liquidation_sum: 200_000 * DOLLARS, // In USD. FIXME: temporary value.
						threshold: Rate::saturating_from_rational(103, 100), // 3%
						liquidation_fee: Rate::saturating_from_rational(105, 100), // 5%
					},
				),
			],
		}
		.assimilate_storage::<Runtime>(&mut storage)
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
			mnt_rate: 10 * DOLLARS,
			mnt_claim_threshold: 0, // disable by default
			minted_pools: vec![DOT, ETH, KSM, BTC],
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		storage.into()
	}
}
