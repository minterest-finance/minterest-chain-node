use crate::{CurrencyId, MntToken, Runtime, DOLLARS};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;
runtime_benchmarks! {
	{ Runtime, mnt_token }

	_ {}
	set_mnt_rate {}: _(RawOrigin::Root, 15 * DOLLARS)
	disable_mnt_minting {}: _(RawOrigin::Root, CurrencyId::BTC)
	enable_mnt_minting {
		 MntToken::disable_mnt_minting(RawOrigin::Root.into(), CurrencyId::DOT).unwrap();
	}: _(RawOrigin::Root, CurrencyId::DOT)
}

// TODO
// #[cfg(test)]
// pub mod tests {
// 	use super::*;
// 	use crate::benchmarking::utils::tests::new_test_ext;
// 	use frame_support::assert_ok;

// 	#[test]
// 	fn test_set_mnt_rate() {
// 		new_test_ext().execute_with(|| {
// 			assert_ok!(test_benchmark_set_mnt_rate());
// 		})
// 	}
// }
