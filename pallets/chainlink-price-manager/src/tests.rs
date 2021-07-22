//! Unit tests for example module.

#![cfg(test)]

use crate::mock::*;
use minterest_primitives::CurrencyId;
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData};
use pallet_traits::PricesManager;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{FixedPointNumber, FixedU128};
use test_helper::currency_mock::*;
use test_helper::users_mock::*;

#[test]
fn create_feed_should_fail() {
	// TODO check admin origin
}

#[test]
fn create_feed_should_work() {
	let oracles_admin: AccountId = 999;
	let oracle1: AccountId = 100;
	let oracle2: AccountId = 200;
	let oracle3: AccountId = 300;
	test_externalities().execute_with(|| {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			3,
			5, // min submissions
			b"MIN-BTC".to_vec(),
			2,
			vec![
				(oracle1, oracles_admin),
				(oracle2, oracles_admin),
				(oracle3, oracles_admin),
			],
			Some(5000),
			None,
		)
		.unwrap();

		let feed_id = 0_u32;
		let feed_created = Event::ChainlinkFeed(pallet_chainlink_feed::Event::FeedCreated(feed_id, ALICE));
		assert!(System::events().iter().any(|record| record.event == feed_created));

		let round_id = 1;
		ChainlinkFeed::submit(Origin::signed(oracle1), 0_u32, round_id, 42 * DOLLARS).unwrap();
		ChainlinkFeed::submit(Origin::signed(oracle2), 0_u32, round_id, 42 * DOLLARS).unwrap();

		// The value is returned only when 3 oracles are subbmited, because min_submissions == 3
		assert_eq!(ChainlinkPriceManager::get_underlying_price(BTC), None);

		let feed_result = ChainlinkFeed::feed(feed_id.into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		assert_eq!(answer, 0);
		ChainlinkFeed::submit(Origin::signed(oracle3), 0_u32, round_id, 42 * DOLLARS).unwrap();

		assert_eq!(
			ChainlinkPriceManager::get_underlying_price(BTC).unwrap(),
			FixedU128::saturating_from_integer(42)
		);
		assert_eq!(ChainlinkPriceManager::get_underlying_price(DOT), None);
	});
}
