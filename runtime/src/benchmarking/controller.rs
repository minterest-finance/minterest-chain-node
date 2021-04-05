use crate::{Balance, CurrencyId, Operation, Rate, Runtime};

use frame_system::RawOrigin;
use orml_benchmarking::{runtime_benchmarks, Zero};
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, controller }

	_ {}

	pause_operation {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Operation::Deposit
	)

	resume_operation {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Operation::Deposit
	)

	set_protocol_interest_factor {
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

	set_protocol_interest_threshold {}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Balance::zero()
	)

	switch_mode {}: _(
		RawOrigin::Root
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use frame_support::assert_ok;

	#[test]
	fn test_pause_operation() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_pause_operation());
		})
	}

	#[test]
	fn test_resume_operation() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_resume_operation());
		})
	}

	#[test]
	fn test_set_protocol_interest_factor() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_protocol_interest_factor());
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
	fn test_set_protocol_interest_threshold() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_protocol_interest_threshold());
		})
	}

	#[test]
	fn test_switch_mode() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_switch_mode());
		})
	}
}
