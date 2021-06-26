//! Unit tests for example module.

#![cfg(test)]

use crate::mock::*;
use frame_support::assert_ok;
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData};
use sp_runtime::traits::AccountIdConversion;

#[test]
fn create_feed_should_fail() {
	// TODO check admin origin
}

#[test]
fn create_feed_should_work() {
	let chainlink_adapter_account: AccountId = ChainlinkPriceAdapterModuleId::get().into_account();
	let oracles_admin: AccountId = 999;
	let oracle1 = 100;
	let oracle2 = 200;
	let oracle3 = 300;
	test_externalities().execute_with(|| {
		ChainlinkPriceAdapter::create_feed(
			admin(),
			20,
			10,
			(10, 1_000),
			3,
			5,
			b"desc".to_vec(),
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
		let feed_id = 0;
		let feed_created = Event::pallet_chainlink_feed(pallet_chainlink_feed::Event::FeedCreated(
			feed_id,
			chainlink_adapter_account,
		));
		assert!(System::events().iter().any(|record| record.event == feed_created));

		let round_id = 1;
		ChainlinkFeed::submit(Origin::signed(oracle1), feed_id, round_id, 42).unwrap();
		ChainlinkFeed::submit(Origin::signed(oracle2), feed_id, round_id, 42).unwrap();
		let feed_result = ChainlinkFeed::feed(feed_id.into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		assert_eq!(answer, 0);
		ChainlinkFeed::submit(Origin::signed(oracle3), feed_id, round_id, 42).unwrap();
		// The value is returned only when 3 oracles are subbmited, because min_submissions == 3
		let feed_result = ChainlinkFeed::feed(feed_id.into()).unwrap();
		let RoundData { answer, .. } = feed_result.latest_data();
		assert_eq!(answer, 42);
	});
}
