//! Unit tests for example module.

#![cfg(test)]

use crate::mock::*;
use minterest_primitives::CurrencyId;
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData};
use sp_runtime::traits::AccountIdConversion;
use test_helper::currency_mock::*;
use test_helper::users_mock::*;

#[test]
fn create_feed_should_fail() {
	// TODO check admin origin
}

#[test]
fn create_feed_should_work() {
	let chainlink_adapter_account: AccountId = ChainlinkPriceManagerPalletId::get().into_account();
	let oracles_admin: AccountId = 999;
	let oracle1: AccountId = 100;
	let oracle2: AccountId = 200;
	let oracle3: AccountId = 300;
	test_externalities().execute_with(|| {
		ChainlinkPriceManager::create_minterest_feed(
			admin_origin(),
			BTC,
			3,
			vec![
				(oracle1, oracles_admin),
				(oracle2, oracles_admin),
				(oracle3, oracles_admin),
			],
		)
		.unwrap();
		let feed_id = 0;
		let feed_created = Event::ChainlinkFeed(pallet_chainlink_feed::Event::FeedCreated(
			feed_id,
			chainlink_adapter_account,
		));
		assert!(System::events().iter().any(|record| record.event == feed_created));

		let round_id = 1;
		ChainlinkPriceManager::submit(Origin::signed(oracle1), BTC, round_id, 42).unwrap();
		ChainlinkPriceManager::submit(Origin::signed(oracle2), BTC, round_id, 42).unwrap();
		let feed_result = ChainlinkFeed::feed(feed_id.into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		assert_eq!(answer, 0);
		ChainlinkPriceManager::submit(Origin::signed(oracle3), BTC, round_id, 42).unwrap();

		// The value is returned only when 3 oracles are subbmited, because min_submissions == 3
		let feed_result = ChainlinkFeed::feed(feed_id.into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		assert_eq!(answer, 42);

		assert_eq!(ChainlinkPriceManager::get_underlying_price(BTC).unwrap(), 42);
		assert_eq!(ChainlinkPriceManager::get_underlying_price(DOT), None);
	});
}
