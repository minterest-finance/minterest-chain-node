use crate::{CurrencyId, EnabledUnderlyingAssetsIds, MinterestOracle, Origin, Price, Prices, Runtime, DOT};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, module_prices }

	_ {}

	lock_price {
		let pool_id: CurrencyId = EnabledUnderlyingAssetsIds::get()[0];

		MinterestOracle::feed_values(RawOrigin::Root.into(), vec![(pool_id, Price::one())])?;
	}: _(
		RawOrigin::Root,
		DOT
	)

	unlock_price {
		let pool_id: CurrencyId = EnabledUnderlyingAssetsIds::get()[0];

		MinterestOracle::feed_values(RawOrigin::Root.into(), vec![(pool_id, Price::one())])?;
		Prices::lock_price(Origin::root(), DOT)?;
	}: _(
		RawOrigin::Root,
		DOT
	)
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use frame_support::assert_ok;

	#[test]
	fn test_lock_price() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_lock_price());
		})
	}

	#[test]
	fn test_unlock_price() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_unlock_price());
		});
	}
}
