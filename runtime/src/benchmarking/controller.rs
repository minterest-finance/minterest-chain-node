use crate::{CurrencyId, Operation, Rate, Runtime};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, controller }

	_ {}

	pause_specific_operation {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Operation::Deposit
	)

	unpause_specific_operation {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Operation::Deposit
	)

	set_insurance_factor {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Rate::one()
	)

	set_max_borrow_rate {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Rate::one()
	)

	set_collateral_factor {}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Rate::one()
	)

	set_borrow_cap {}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Some(0u128)
	)

	switch_mode {}: _(
		RawOrigin::Root
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;
	use frame_support::pallet_prelude::GenesisBuild;
	use liquidity_pools::Pool;
	use minterest_primitives::Balance;
	use sp_runtime::traits::Zero;
	use sp_runtime::{FixedPointNumber, FixedU128};

	fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();
		liquidity_pools::GenesisConfig::<Runtime> {
			pools: vec![(
				CurrencyId::DOT,
				Pool {
					total_borrowed: Balance::zero(),
					borrow_index: FixedU128::one(),
					total_insurance: Balance::zero(),
				},
			)],
			pool_user_data: vec![],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}

	#[test]
	fn test_pause_specific_operation() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_pause_specific_operation());
		})
	}

	#[test]
	fn test_unpause_specific_operation() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_unpause_specific_operation());
		})
	}

	#[test]
	fn test_set_insurance_factor() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_insurance_factor());
		})
	}

	#[test]
	fn test_set_max_borrow_rate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_max_borrow_rate());
		})
	}

	#[test]
	fn test_set_collateral_factor() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_collateral_factor());
		})
	}

	#[test]
	fn test_set_borrow_cap() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_borrow_cap());
		})
	}

	#[test]
	fn test_switch_mode() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_switch_mode());
		})
	}
}
