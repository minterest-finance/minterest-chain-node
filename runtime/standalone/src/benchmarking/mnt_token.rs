use super::utils::{create_pools, prepare_for_mnt_distribution};
use crate::{MntToken, OriginalAsset, OriginalAsset::DOT, Runtime, System, DOLLARS};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;

runtime_benchmarks! {
	{ Runtime, mnt_token }

	set_speed {
		create_pools();
		prepare_for_mnt_distribution(OriginalAsset::get_original_assets())?;
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
