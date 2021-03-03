//! Unit tests for the prices module.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn get_underlying_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Price 1 BTC = 48000 USD
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(48000u128))
		);
		// Price 1 DOT = 40 USD
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::DOT),
			Some(Price::saturating_from_integer(40u128))
		);

		assert_eq!(PricesModule::get_underlying_price(CurrencyId::MNT), Some(Price::zero()));

		assert_eq!(PricesModule::get_underlying_price(CurrencyId::MDOT), None);
	});
}

#[test]
fn lock_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(48_000))
		);
		LockedPrice::<Test>::insert(CurrencyId::BTC, Price::saturating_from_integer(80_000));
		assert_eq!(
			PricesModule::get_underlying_price(CurrencyId::BTC),
			Some(Price::saturating_from_integer(80_000))
		);
	});
}

#[test]
fn lock_price_call_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
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
		assert_noop!(PricesModule::lock_price(bob(), CurrencyId::BTC), BadOrigin);
		assert_noop!(
			PricesModule::lock_price(alice(), CurrencyId::MDOT),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn unlock_price_call_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		LockedPrice::<Test>::insert(CurrencyId::BTC, Price::saturating_from_integer(80000));
		assert_ok!(PricesModule::unlock_price(alice(), CurrencyId::BTC));

		let unlock_price_event = Event::module_prices(crate::Event::UnlockPrice(CurrencyId::BTC));
		assert!(System::events().iter().any(|record| record.event == unlock_price_event));

		assert_eq!(PricesModule::locked_price(CurrencyId::BTC), None);

		assert_noop!(PricesModule::unlock_price(bob(), CurrencyId::BTC), BadOrigin);
		assert_noop!(
			PricesModule::lock_price(alice(), CurrencyId::MDOT),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}
