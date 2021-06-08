use super::utils::{create_pools, prepare_for_mnt_distribution};
use crate::{EnabledUnderlyingAssetsIds, MntToken, Runtime, System, BTC, DOLLARS, DOT};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, mnt_token }

	_ {}

	disable_mnt_minting {
		let pools = EnabledUnderlyingAssetsIds::get();
		create_pools(&pools);
		prepare_for_mnt_distribution(pools)?;
		System::set_block_number(10);
	}: _(RawOrigin::Root, BTC)

	enable_mnt_minting {
		let pools = EnabledUnderlyingAssetsIds::get();
		create_pools(&pools);
		prepare_for_mnt_distribution(pools)?;
		System::set_block_number(10);
		MntToken::disable_mnt_minting(RawOrigin::Root.into(), DOT)?;
		System::set_block_number(11);
	}: _(RawOrigin::Root, DOT, 10 * DOLLARS)

	update_speed {
		let pools = EnabledUnderlyingAssetsIds::get();
		create_pools(&pools);
		prepare_for_mnt_distribution(pools)?;
		System::set_block_number(10);
		MntToken::disable_mnt_minting(RawOrigin::Root.into(), DOT)?;
		MntToken::enable_mnt_minting(RawOrigin::Root.into(), DOT, 1 * DOLLARS)?;
		System::set_block_number(11);
	}: _(RawOrigin::Root, DOT, 10 * DOLLARS)
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_disable_mnt_minting() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_disable_mnt_minting());
		})
	}

	#[test]
	fn test_enable_mnt_minting() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_enable_mnt_minting());
		})
	}

	#[test]
	fn test_update_speed() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_update_speed());
		})
	}
}
