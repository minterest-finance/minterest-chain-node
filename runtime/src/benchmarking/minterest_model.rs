use crate::{CurrencyId, Runtime};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, minterest_model }

	_ {}

	set_jump_multiplier_per_block {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		10u128,
		10u128
	)

	set_base_rate_per_block {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		10u128,
		10u128
	)

	set_multiplier_per_block {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		10u128,
		10u128
	)

	set_kink {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		10u128,
		10u128
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;

	fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}

	#[test]
	fn test_set_jump_multiplier_per_block() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_jump_multiplier_per_block());
		})
	}

	#[test]
	fn test_set_base_rate_per_block() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_base_rate_per_block());
		})
	}

	#[test]
	fn test_set_multiplier_per_block() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_multiplier_per_block());
		})
	}

	#[test]
	fn test_set_kink() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_multiplier_per_block());
		})
	}
}
