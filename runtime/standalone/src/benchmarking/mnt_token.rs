use super::utils::{create_pools, prepare_for_mnt_distribution};
use crate::{EnabledUnderlyingAssetsIds, MntToken, Runtime, System, DOLLARS, DOT};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;

runtime_benchmarks! {
	{ Runtime, mnt_token }

	set_speed {
		let pools = EnabledUnderlyingAssetsIds::get();
		create_pools(&pools);
		prepare_for_mnt_distribution(pools)?;
		System::set_block_number(10);
		MntToken::set_speed(RawOrigin::Root.into(), DOT, 1)?;
		System::set_block_number(11);
	}: _(RawOrigin::Root, DOT, 10 * DOLLARS)
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_set_speed() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_speed());
		})
	}
}
