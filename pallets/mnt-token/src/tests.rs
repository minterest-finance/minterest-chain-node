#![cfg(test)]

use super::Error;
use crate::mock::*;

use frame_support::{assert_noop, assert_ok};
use minterest_primitives::{CurrencyId, CurrencyPair, Rate};
use sp_arithmetic::FixedPointNumber;

#[test]
fn test_set_mnt_rate() {
	new_test_ext().execute_with(|| {
		// TODO remove code duplication
		let old_rate = Rate::zero();
		let new_rate = Rate::saturating_from_rational(11, 10);
		assert_eq!(MntToken::mnt_rate(), old_rate);
		assert_ok!(MntToken::set_mnt_rate(admin(), new_rate));
		assert_eq!(MntToken::mnt_rate(), new_rate);
		let new_mnt_rate_event = Event::mnt_token(crate::Event::NewMntRate(old_rate, new_rate));
		assert!(System::events().iter().any(|record| record.event == new_mnt_rate_event));

		let old_rate = new_rate;
		let new_rate = Rate::saturating_from_rational(12, 10);
		assert_eq!(MntToken::mnt_rate(), old_rate);
		assert_ok!(MntToken::set_mnt_rate(admin(), new_rate));
		assert_eq!(MntToken::mnt_rate(), new_rate);
		let new_mnt_rate_event = Event::mnt_token(crate::Event::NewMntRate(old_rate, new_rate));
		assert!(System::events().iter().any(|record| record.event == new_mnt_rate_event));
	});
}

#[test]
fn test_market_list_manipulation() {
	new_test_ext().execute_with(|| {
		// Add new market
		let new_market = CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT);
		assert_ok!(MntToken::add_market(admin(), new_market));
		let new_market_event = Event::mnt_token(crate::Event::NewMarketListed(new_market));
		assert!(System::events().iter().any(|record| record.event == new_market_event));
		assert_eq!(MntToken::mnt_markets().len(), 1);

		// Try to add the same market
		assert_noop!(
			MntToken::add_market(admin(), new_market),
			Error::<Runtime>::MarketAlreadyExists
		);
		assert_eq!(MntToken::mnt_markets().len(), 1);

		// Add second market
		let new_market2 = CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM);
		assert_ok!(MntToken::add_market(admin(), new_market2));
		let new_market_event = Event::mnt_token(crate::Event::NewMarketListed(new_market2));
		assert!(System::events().iter().any(|record| record.event == new_market_event));
		assert_eq!(MntToken::mnt_markets().len(), 2);

		// Remove first market
		assert_ok!(MntToken::remove_market(admin(), new_market));
		let remove_market_event = Event::mnt_token(crate::Event::MarketRemoved(new_market));
		assert!(System::events()
			.iter()
			.any(|record| record.event == remove_market_event));
		assert_eq!(MntToken::mnt_markets().len(), 1);

		// Try to remove not exist market (already removed)
		assert_noop!(
			MntToken::remove_market(admin(), new_market),
			Error::<Runtime>::MarketNotExists
		);
		assert_eq!(MntToken::mnt_markets().len(), 1);
	});
}
