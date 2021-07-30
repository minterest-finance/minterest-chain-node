//! Unit tests for example module.

#![cfg(test)]

use crate::mock::*;
use codec::Decode;
use frame_support::assert_ok;
use minterest_primitives::{currency::CurrencyType::UnderlyingAsset, CurrencyId};
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData};
use pallet_traits::PricesManager;
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
};
use sp_runtime::{FixedPointNumber, FixedU128};
use test_helper::{currency_mock::*, users_mock::*};

fn create_default_feeds() {
	for currency in CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset) {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			ChainlinkPriceManager::convert_to_description(currency).to_vec(),
			0,
			vec![(ORACLE, ORACLES_ADMIN)],
			None,
			None,
		)
		.unwrap();
	}
}

#[test]
fn offchain_worker_test() {
	let mut ext = test_externalities();
	let (offchain, _) = TestOffchainExt::new();
	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.execute_with(|| {
		create_default_feeds();
		assert_ok!(ChainlinkPriceManager::_offchain_worker(3));

		// There are 4 pools enabled therefore must be 4 events for each pool
		assert_eq!(trans_pool_state.read().transactions.len(), 4);
		let transaction = trans_pool_state.write().transactions.pop().unwrap();

		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		// Just check the first event and ignore others
		let (_feed_id, round_id) = match ex.call {
			crate::mock::Call::ChainlinkPriceManager(crate::Call::initiate_new_round(feed_id, round_id)) => {
				(feed_id, round_id)
			}
			e => panic!("Unexpected call: {:?}", e),
		};
		assert_eq!(round_id, 1);
	});
}

#[test]
fn get_feed_id() {
	let oracle: AccountId = 100;
	test_externalities().execute_with(|| {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			ChainlinkPriceManager::convert_to_description(BTC).to_vec(),
			0,
			vec![(oracle, ORACLES_ADMIN)],
			None,
			None,
		)
		.unwrap();
		assert_eq!(ChainlinkPriceManager::get_feed_id(BTC).unwrap(), 0);
		assert_eq!(ChainlinkPriceManager::get_feed_id(DOT), None);
	});
}

#[test]
fn create_feed_should_work() {
	let oracle1: AccountId = 100;
	let oracle2: AccountId = 200;
	let oracle3: AccountId = 300;
	test_externalities().execute_with(|| {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			3, // min submissions
			5,
			ChainlinkPriceManager::convert_to_description(BTC).to_vec(),
			2,
			vec![
				(oracle1, ORACLES_ADMIN),
				(oracle2, ORACLES_ADMIN),
				(oracle3, ORACLES_ADMIN),
			],
			None,
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
