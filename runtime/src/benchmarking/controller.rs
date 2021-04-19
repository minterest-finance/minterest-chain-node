use crate::{Balance, Operation, Rate, Runtime, DOT};

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
		DOT,
		Operation::Deposit
	)

	resume_operation {
	}: _(
		RawOrigin::Root,
		DOT,
		Operation::Deposit
	)

	set_protocol_interest_factor {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_max_borrow_rate {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_collateral_factor {}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_borrow_cap {}: _(
		RawOrigin::Root,
		DOT,
		Some(0u128)
	)

	set_protocol_interest_threshold {}: _(
		RawOrigin::Root,
		DOT,
		Balance::zero()
	)

	switch_whitelist_mode {}: _(
		RawOrigin::Root
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_pause_operation() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_pause_operation());
		})
	}

	#[test]
	fn test_resume_operation() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_resume_operation());
		})
	}

	#[test]
	fn test_set_protocol_interest_factor() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_protocol_interest_factor());
		})
	}

	#[test]
	fn test_set_max_borrow_rate() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_max_borrow_rate());
		})
	}

	#[test]
	fn test_set_collateral_factor() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_collateral_factor());
		})
	}

	#[test]
	fn test_set_borrow_cap() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_borrow_cap());
		})
	}

	#[test]
	fn test_set_protocol_interest_threshold() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_protocol_interest_threshold());
		})
	}

	#[test]
	fn test_switch_whitelist_mode() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_switch_whitelist_mode());
		})
	}
}
