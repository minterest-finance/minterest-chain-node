use crate::{AccountId, CurrencyId, Operation, Runtime};

use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

const SEED: u32 = 0;

runtime_benchmarks! {
	{ Runtime, controller }

	_ {}

	pause_specific_operation {
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT,
		Operation::Deposit
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
	fn test_pause_specific_operation() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_collateral_params());
		})
	}
}
