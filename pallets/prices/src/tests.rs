//! Unit tests for the prices module.

use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};
use minterest_primitives::{Price, OriginalAsset};
use pallet_traits::PricesManager;
use sp_runtime::{
	traits::{BadOrigin, Zero},
	FixedPointNumber,
};

#[test]
fn get_underlying_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Price 1 BTC = 48000 USD
		assert_eq!(
			TestPrices::get_underlying_price(BTC),
			Some(Price::saturating_from_integer(48000u128))
		);
		// Price 1 DOT = 40 USD
		assert_eq!(
			TestPrices::get_underlying_price(DOT),
			Some(Price::saturating_from_integer(40u128))
		);

		assert_eq!(TestPrices::get_underlying_price(OriginalAsset::MNT), Some(Price::zero()));
	});
}

#[test]
fn lock_price_should_work() {
	ExtBuilder::default()
		.set_locked_price(BTC, Price::saturating_from_integer(80_000))
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPrices::get_underlying_price(BTC),
				Some(Price::saturating_from_integer(80_000))
			);
			assert_ok!(TestPrices::unlock_price(alice_origin(), BTC));
			assert_eq!(
				TestPrices::get_underlying_price(BTC),
				Some(Price::saturating_from_integer(48_000))
			);
		});
}

#[test]
fn lock_price_call_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(TestPrices::lock_price(alice_origin(), BTC));

		let lock_price_event = Event::TestPrices(crate::Event::LockPrice(BTC, Price::saturating_from_integer(48000)));
		assert!(System::events().iter().any(|record| record.event == lock_price_event));
		assert_eq!(
			TestPrices::locked_price_storage(BTC),
			Some(Price::saturating_from_integer(48000))
		);
		assert_noop!(TestPrices::lock_price(bob_origin(), BTC), BadOrigin);
	});
}

#[test]
fn unlock_price_call_should_work() {
	ExtBuilder::default()
		.set_locked_price(BTC, Price::saturating_from_integer(80000))
		.build()
		.execute_with(|| {
			System::set_block_number(1);
			assert_ok!(TestPrices::unlock_price(alice_origin(), BTC));

			let unlock_price_event = Event::TestPrices(crate::Event::UnlockPrice(BTC));
			assert!(System::events().iter().any(|record| record.event == unlock_price_event));

			assert_eq!(TestPrices::locked_price_storage(BTC), None);

			assert_noop!(TestPrices::unlock_price(bob_origin(), BTC), BadOrigin);
		});
}
