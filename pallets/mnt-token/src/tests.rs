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

		// Try to remove not exist market (that already removed)
		assert_noop!(
			MntToken::remove_market(admin(), new_market),
			Error::<Runtime>::MarketNotExists
		);
		assert_eq!(MntToken::mnt_markets().len(), 1);
	});
}

#[test]
fn test_get_listed_market_utilities() {
	new_test_ext().execute_with(|| {
		let dot_market = CurrencyPair::new(CurrencyId::DOT, CurrencyId::MDOT);
		assert_ok!(MntToken::add_market(admin(), dot_market));
		let eth_market = CurrencyPair::new(CurrencyId::ETH, CurrencyId::METH);
		assert_ok!(MntToken::add_market(admin(), eth_market));
		let ksm_market = CurrencyPair::new(CurrencyId::KSM, CurrencyId::MKSM);
		assert_ok!(MntToken::add_market(admin(), ksm_market));
		let btc_market = CurrencyPair::new(CurrencyId::BTC, CurrencyId::MBTC);
		assert_ok!(MntToken::add_market(admin(), btc_market));
		assert_eq!(MntToken::mnt_markets().len(), 4);

		// Amount tokens: 50 for each market
		// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
		// Expected utilities results: DOT = 25, ETH = 75, KSM = 100, BTC = 150
		let (markets_result, total_utility) = MntToken::get_listed_markets_utilities().unwrap();
		assert_eq!(markets_result.len(), MntToken::mnt_markets().len());
		assert_eq!(markets_result[0], (dot_market, 25));
		assert_eq!(markets_result[1], (eth_market, 75));
		assert_eq!(markets_result[2], (ksm_market, 100));
		assert_eq!(markets_result[3], (btc_market, 150));
		assert_eq!(total_utility, 350);
	});
}

#[test]
fn get_get_listed_market_utilities_fail() {
	new_test_ext().execute_with(|| {
		// Not listed in liquidity pool market.
		// get_underlying_price should fail
		let not_listed_market = CurrencyPair::new(CurrencyId::MNT, CurrencyId::MDOT);
		assert_ok!(MntToken::add_market(admin(), not_listed_market));
		assert_noop!(
			MntToken::get_listed_markets_utilities(),
			Error::<Runtime>::GetUnderlyingPriceFail
		);
	});
}
