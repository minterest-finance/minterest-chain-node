#![cfg(test)]

use super::Error;
use crate::mock::*;

use frame_support::{assert_noop, assert_ok};
use minterest_primitives::{CurrencyId, Rate};
use sp_arithmetic::FixedPointNumber;

const KSM: CurrencyId = CurrencyId::KSM;
const DOT: CurrencyId = CurrencyId::DOT;
const ETH: CurrencyId = CurrencyId::ETH;
const BTC: CurrencyId = CurrencyId::BTC;

const MAX_RATE_ACCURACY: i64 = 100000000000000000;

#[test]
fn test_mnt_speed_calculation() {
	new_test_ext().execute_with(|| {
		assert_ok!(MntToken::enable_mnt_minting(admin(), DOT));
		assert_ok!(MntToken::enable_mnt_minting(admin(), KSM));
		assert_ok!(MntToken::enable_mnt_minting(admin(), ETH));
		assert_ok!(MntToken::enable_mnt_minting(admin(), BTC));
		let mnt_rate = Rate::saturating_from_integer(10);
		assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));

		// Formula:
		// asset_price = oracle.get_underlying_price(mtoken)});
		// utility = m_tokens_total_borrows * asset_price
		// utility_fraction = utility / sum_of_all_pools_utilities
		// pool_mnt_speed = mnt_rate * utility_fraction

		// Input parameters:
		// mnt_rate: 10
		// Amount total borrowed tokens: 50 for each pool
		// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
		// utilities: DOT = 25, ETH = 75, KSM = 100, BTC = 150
		// sum_of_all_pools_utilities = 350

		// DOT
		// utility_fraction = 25 / 350 = 0.071428571428571428
		// mnt_speed = utility_fraction * mnt_rate = 0.714285714285714280
		let expected_dot_mnt_speed = Rate::saturating_from_rational(71428571428571428_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(DOT), Some(expected_dot_mnt_speed));

		// KSM
		// utility_ftaction = 100 / 350 = 0.285714285714285714
		// mnt_speed = utility_fraction * mnt_rate = 2.85714285714285714
		let expected_ksm_mnt_speed = Rate::saturating_from_rational(285714285714285714_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(KSM), Some(expected_ksm_mnt_speed));

		// ETH
		// utility_ftaction = 75 / 350 = 0.214285714285714285
		// mnt_speed = utility_fraction * mnt_rate = 2.14285714285714285
		let expected_eth_mnt_speed = Rate::saturating_from_rational(214285714285714285_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(ETH), Some(expected_eth_mnt_speed));

		// BTC
		// utility_ftaction = 150 / 350 = 0.428571428571428571
		// mnt_speed = utility_fraction * mnt_rate = 4.28571428571428571
		let expected_btc_mnt_speed = Rate::saturating_from_rational(428571428571428571_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(BTC), Some(expected_btc_mnt_speed));

		let sum = expected_dot_mnt_speed + expected_btc_mnt_speed + expected_eth_mnt_speed + expected_ksm_mnt_speed;
		// Sum of all mnt_speeds is equal to mnt_rate
		assert_eq!(sum.round(), mnt_rate);

		// Multiply mnt rate in twice. Expected mnt speeds should double up
		let mnt_rate = Rate::saturating_from_integer(20);
		assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));
		assert_eq!(
			MntToken::mnt_speeds(DOT),
			Some(expected_dot_mnt_speed * Rate::saturating_from_integer(2))
		);
		assert_eq!(
			MntToken::mnt_speeds(KSM),
			Some(expected_ksm_mnt_speed * Rate::saturating_from_integer(2))
		);
		assert_eq!(
			MntToken::mnt_speeds(ETH),
			Some(expected_eth_mnt_speed * Rate::saturating_from_integer(2))
		);
		assert_eq!(
			MntToken::mnt_speeds(BTC),
			Some(expected_btc_mnt_speed * Rate::saturating_from_integer(2))
		);
	});
}

#[test]
fn test_disable_mnt_minting() {
	// 1. Disable MNT minting for one pool and check mnt speeds recalculation
	// 2. Enable MNT minting for disabled pool
	new_test_ext_with_prepared_mnt_speeds().execute_with(|| {
		// Make sure that speeds were precalculated
		// mnt_rate == 10
		let expected_dot_mnt_speed = Rate::saturating_from_rational(71428571428571428_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(DOT), Some(expected_dot_mnt_speed));

		// Disable MNT minting for BTC
		assert_ok!(MntToken::disable_mnt_minting(admin(), BTC));
		assert_eq!(MntToken::mnt_speeds(BTC), None);
		// Now utilities: DOT = 25, ETH = 75, KSM = 100
		// sum_of_all_pools_utilities = 200

		// DOT mnt_speed = 25 / 200 * 10 = 1.25
		let expected_dot_mnt_speed = Rate::saturating_from_rational(125, 100);
		assert_eq!(MntToken::mnt_speeds(DOT), Some(expected_dot_mnt_speed));

		// ETH mnt_speed 75 / 200 * 10 = 3.75
		let expected_eth_mnt_speed = Rate::saturating_from_rational(375, 100);
		assert_eq!(MntToken::mnt_speeds(ETH), Some(expected_eth_mnt_speed));

		// KSM mnt_speed = 100 / 200 * 10 = 5
		let expected_ksm_mnt_speed = Rate::saturating_from_integer(5);
		assert_eq!(MntToken::mnt_speeds(KSM), Some(expected_ksm_mnt_speed));

		// Enable MNT minting for BTC.
		// MNT speeds should be recalculated again
		assert_ok!(MntToken::enable_mnt_minting(admin(), BTC));
		let expected_dot_mnt_speed = Rate::saturating_from_rational(71428571428571428_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(DOT), Some(expected_dot_mnt_speed));
		let expected_ksm_mnt_speed = Rate::saturating_from_rational(285714285714285714_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(KSM), Some(expected_ksm_mnt_speed));
		let expected_eth_mnt_speed = Rate::saturating_from_rational(214285714285714285_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(ETH), Some(expected_eth_mnt_speed));
		let expected_btc_mnt_speed = Rate::saturating_from_rational(428571428571428571_i64, MAX_RATE_ACCURACY);
		assert_eq!(MntToken::mnt_speeds(BTC), Some(expected_btc_mnt_speed));
	});
}

#[test]
fn test_disable_generating_all_mnt_tokens() {
	new_test_ext_with_prepared_mnt_speeds().execute_with(|| {
		let zero = Rate::zero();
		assert_ok!(MntToken::set_mnt_rate(admin(), zero));
		assert_eq!(MntToken::mnt_speeds(DOT), Some(zero));
		assert_eq!(MntToken::mnt_speeds(KSM), Some(zero));
		assert_eq!(MntToken::mnt_speeds(ETH), Some(zero));
		assert_eq!(MntToken::mnt_speeds(BTC), Some(zero));
	});
}

#[test]
fn test_set_mnt_rate() {
	new_test_ext().execute_with(|| {
		let test = |new_rate: Rate| {
			let old_rate = MntToken::mnt_rate();
			assert_eq!(MntToken::mnt_rate(), old_rate);
			assert_ok!(MntToken::set_mnt_rate(admin(), new_rate));
			assert_eq!(MntToken::mnt_rate(), new_rate);
			let new_mnt_rate_event = Event::mnt_token(crate::Event::NewMntRate(old_rate, new_rate));
			assert!(System::events().iter().any(|record| record.event == new_mnt_rate_event));
		};

		test(Rate::saturating_from_rational(11, 10));
		test(Rate::saturating_from_rational(12, 10));
	});
}

#[test]
fn test_minting_enable_disable() {
	new_test_ext().execute_with(|| {
		// Add new mnt minting
		assert_ok!(MntToken::enable_mnt_minting(admin(), DOT));
		let new_minting_event = Event::mnt_token(crate::Event::MntMintingEnabled(DOT));
		assert!(System::events().iter().any(|record| record.event == new_minting_event));
		assert_ne!(MntToken::mnt_speeds(DOT), None);
		// Try to add the same pool
		assert_noop!(
			MntToken::enable_mnt_minting(admin(), DOT),
			Error::<Runtime>::MntMintingAlreadyEnabled
		);

		// Add minting for another one pool
		assert_ok!(MntToken::enable_mnt_minting(admin(), KSM));
		let new_minting_event = Event::mnt_token(crate::Event::MntMintingEnabled(KSM));
		assert!(System::events().iter().any(|record| record.event == new_minting_event));
		assert_ne!(MntToken::mnt_speeds(KSM), None);

		// Disable MNT minting for DOT
		assert_ok!(MntToken::disable_mnt_minting(admin(), DOT));
		let disable_mnt_minting_event = Event::mnt_token(crate::Event::MntMintingDisabled(DOT));
		assert!(System::events()
			.iter()
			.any(|record| record.event == disable_mnt_minting_event));
		assert_eq!(MntToken::mnt_speeds(DOT), None);

		// Try to disable minting that wasn't enabled
		assert_noop!(
			MntToken::disable_mnt_minting(admin(), DOT),
			Error::<Runtime>::MntMintingNotEnabled,
		);
	});
}

#[test]
fn test_calculate_enabled_pools_utilities() {
	new_test_ext().execute_with(|| {
		assert_ok!(MntToken::enable_mnt_minting(admin(), DOT));
		assert_ok!(MntToken::enable_mnt_minting(admin(), ETH));
		assert_ok!(MntToken::enable_mnt_minting(admin(), KSM));
		assert_ok!(MntToken::enable_mnt_minting(admin(), BTC));
		assert_ne!(MntToken::mnt_speeds(DOT), None);
		assert_ne!(MntToken::mnt_speeds(ETH), None);
		assert_ne!(MntToken::mnt_speeds(KSM), None);
		assert_ne!(MntToken::mnt_speeds(BTC), None);

		// Amount tokens: 50 for each currency
		// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
		// Expected utilities results: DOT = 25, ETH = 75, KSM = 100, BTC = 150
		let (currency_utilities, total_utility) = MntToken::calculate_enabled_pools_utilities().unwrap();
		assert!(currency_utilities.contains(&(DOT, 25)));
		assert!(currency_utilities.contains(&(ETH, 75)));
		assert!(currency_utilities.contains(&(KSM, 100)));
		assert!(currency_utilities.contains(&(BTC, 150)));
		assert_eq!(total_utility, 350);
	});
}

#[test]
fn test_calculate_enabled_pools_utilities_fail() {
	new_test_ext().execute_with(|| {
		let non_existent_liquidity_pool = CurrencyId::MNT;
		assert_noop!(
			MntToken::enable_mnt_minting(admin(), non_existent_liquidity_pool),
			Error::<Runtime>::NotValidUnderlyingAssetId
		);
	});
}
