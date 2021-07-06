use crate::{Rate, Runtime, DOT};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_runtime::traits::One;

runtime_benchmarks! {
	{ Runtime, minterest_model }

	set_jump_multiplier {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_base_rate {
	}: _(
		RawOrigin::Root,
		DOT,
		Rate::one()
	)

	set_multiplier {
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
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_set_jump_multiplier_per_block() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_jump_multiplier());
		})
	}

	#[test]
	fn test_set_base_rate_per_block() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_base_rate());
		})
	}

	#[test]
	fn test_set_multiplier_per_block() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_multiplier());
		})
	}

	#[test]
	fn test_set_kink() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_multiplier());
		})
	}
}
