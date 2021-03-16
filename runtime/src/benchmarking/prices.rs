use crate::{CurrencyId, EnabledUnderlyingAssetId, MinterestOracle, Origin, Price, Prices, Runtime};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_runtime::FixedPointNumber;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, module_prices }

	_ {}

	lock_price {
		let pool_id: CurrencyId = EnabledUnderlyingAssetId::get()[0];

		MinterestOracle::feed_values(RawOrigin::Root.into(), vec![(pool_id, Price::one())])?;
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT
	)

	unlock_price {
		let pool_id: CurrencyId = EnabledUnderlyingAssetId::get()[0];

		MinterestOracle::feed_values(RawOrigin::Root.into(), vec![(pool_id, Price::one())])?;
		Prices::lock_price(Origin::root(), CurrencyId::DOT)?;
	}: _(
		RawOrigin::Root,
		CurrencyId::DOT
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
