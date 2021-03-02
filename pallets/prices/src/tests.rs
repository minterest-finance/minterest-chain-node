//! Unit tests for the prices module.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn get_underlying_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Price = 48000 USD, right shift the decimal point (18-8) places
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(48000_0000000000u128))
		);
		// Price = 40 USD, right shift the decimal point (18-10) places
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::DOT),
			Some(Price::saturating_from_integer(40_00000000u128))
		);

		assert_eq!(PricesModule::get_underlying_price(CurrencyId::MNT), Some(Price::zero()));

		assert_eq!(PricesModule::get_underlying_price(CurrencyId::MDOT), None);
	});
}

#[test]
fn get_relative_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			PricesModule::get_relative_price(CurrencyId::BTC, CurrencyId::DOT),
			// 1 BTC = 1200 DOT, right shift the decimal point (12-10) places
			Some(Price::saturating_from_integer(1200_00))
		);
		assert_eq!(
			PricesModule::get_relative_price(CurrencyId::ETH, CurrencyId::DOT),
			// 1 DOT = 48000 * 10^10, 1 ETH = 1500.
			// 1 ETH = 37.5 DOT, left shift the decimal point 10 places
			Some(Price::saturating_from_rational(375, 1000000000))
		);
		assert_eq!(
			// 1 BTC = 48000 * 10^10, 1 KSM = 250.
			// 1 BTC = 192 KSM, right shift the decimal point 10 places
			PricesModule::get_relative_price(CurrencyId::BTC, CurrencyId::KSM),
			Some(Price::saturating_from_integer(192_0000000000_u128))
		);
		assert_eq!(
			PricesModule::get_relative_price(CurrencyId::BTC, CurrencyId::BTC),
			Some(Price::saturating_from_rational(1, 1)) // 1 BTC = 1 BTC,
		);
		assert_eq!(PricesModule::get_relative_price(CurrencyId::DOT, CurrencyId::MNT), None);
	});
}

#[test]
fn lock_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(48000_0000000000u128))
		);
		LockedPrice::<Test>::insert(CurrencyId::BTC, Price::saturating_from_integer(80000));
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(800000000000000u128))
		);
	});
}

#[test]
fn lock_price_call_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(PricesModule::lock_price(bob(), CurrencyId::BTC), BadOrigin);
		assert_ok!(PricesModule::lock_price(alice(), CurrencyId::BTC));

		let lock_price_event = Event::module_prices(crate::Event::LockPrice(
			CurrencyId::BTC,
			Price::saturating_from_integer(48000),
		));
		assert!(System::events().iter().any(|record| record.event == lock_price_event));
		assert_eq!(
			PricesModule::locked_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(48000))
		);
	});
}

#[test]
fn unlock_price_call_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		LockedPrice::<Test>::insert(CurrencyId::BTC, Price::saturating_from_integer(80000));
		assert_noop!(PricesModule::unlock_price(bob(), CurrencyId::BTC), BadOrigin);
		assert_ok!(PricesModule::unlock_price(alice(), CurrencyId::BTC));

		let unlock_price_event = Event::module_prices(crate::Event::UnlockPrice(CurrencyId::BTC));
		assert!(System::events().iter().any(|record| record.event == unlock_price_event));

		assert_eq!(PricesModule::locked_price(CurrencyId::BTC), None);
	});
}
