use super::utils::prepare_for_mnt_distribution;
use crate::{EnabledUnderlyingAssetsIds, MntToken, Runtime, System, BTC, DOLLARS, DOT};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, mnt_token }

	_ {}
	set_mnt_rate {
		prepare_for_mnt_distribution(EnabledUnderlyingAssetsIds::get())?;
		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;
		System::set_block_number(11);
	}: _(RawOrigin::Root, 15 * DOLLARS)

	disable_mnt_minting {
		prepare_for_mnt_distribution(EnabledUnderlyingAssetsIds::get())?;
		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;
		System::set_block_number(11);
	}: _(RawOrigin::Root, BTC)

	enable_mnt_minting {
		prepare_for_mnt_distribution(EnabledUnderlyingAssetsIds::get())?;
		System::set_block_number(10);
		MntToken::refresh_mnt_speeds()?;
		MntToken::disable_mnt_minting(RawOrigin::Root.into(), DOT)?;
		System::set_block_number(11);
	}: _(RawOrigin::Root, DOT)
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_set_mnt_rate() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_set_mnt_rate());
		})
	}

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
}
