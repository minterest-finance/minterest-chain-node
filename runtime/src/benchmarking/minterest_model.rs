use crate::{Rate, Runtime, DOT};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, minterest_model }

	_ {}

	set_jump_multiplier_per_year {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_base_rate_per_year {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_multiplier_per_year {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_kink {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use frame_support::assert_ok;

	#[test]
	fn test_set_jump_multiplier_per_block() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_jump_multiplier_per_year());
		})
	}

	#[test]
	fn test_set_base_rate_per_block() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_base_rate_per_year());
		})
	}

	#[test]
	fn test_set_multiplier_per_block() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_multiplier_per_year());
		})
	}

	#[test]
	fn test_set_kink() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_multiplier_per_year());
		})
	}
}
