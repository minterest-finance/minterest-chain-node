use crate::Runtime;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;

runtime_benchmarks! {
	{ Runtime, chainlink_price_manager }

	disable_feeding {
	}: _(
		RawOrigin::Root
	)

	enable_feeding {
	}: _(
		RawOrigin::Root
	)

	initiate_new_round {
	}: _(
		RawOrigin::None,
		0, // feed_id
		1 // round_id
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_disable_feeding() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_disable_feeding());
		})
	}

	#[test]
	fn test_enable_feeding() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_enable_feeding());
		})
	}

	#[test]
	fn test_initiate_new_round() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_initiate_new_round());
		})
	}
}
