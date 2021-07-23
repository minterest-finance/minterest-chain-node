//! Unit tests for example module.

#![cfg(test)]

use crate::mock::*;
use frame_support::assert_ok;
use minterest_primitives::CurrencyId;
use pallet_chainlink_feed::{FeedInterface, FeedOracle, RoundData};
use pallet_traits::PricesManager;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::{FixedPointNumber, FixedU128};
use test_helper::currency_mock::*;
use test_helper::users_mock::*;

use codec::{Decode, Encode};
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
};

#[test]
fn offchain_worker_test() {
	let oracles_admin: AccountId = 999;
	let oracle1: AccountId = 100;
	let mut ext = test_externalities();
	let (offchain, _) = TestOffchainExt::new();
	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.execute_with(|| {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-BTC".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-ETH".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-DOT".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-KSM".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		assert_ok!(ChainlinkPriceManager::_offchain_worker(3));

		// 1 balancing transcation in transactions pool
		assert_eq!(trans_pool_state.read().transactions.len(), 1);
		let transaction = trans_pool_state.write().transactions.pop().unwrap();

		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		// Called extrinsic input params
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
fn get_min_round_id() {
	let oracles_admin: AccountId = 999;
	let oracle1: AccountId = 100;
	test_externalities().execute_with(|| {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-BTC".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-ETH".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-DOT".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-KSM".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
			None,
			None,
		)
		.unwrap();

		let min_round_id = ChainlinkPriceManager::get_min_round_id().unwrap();
		assert_eq!(min_round_id, 0);

		ChainlinkFeed::submit(
			Origin::signed(oracle1),
			ChainlinkPriceManager::get_feed_id(BTC).unwrap(),
			min_round_id + 1,
			42 * DOLLARS,
		)
		.unwrap();
		ChainlinkFeed::submit(
			Origin::signed(oracle1),
			ChainlinkPriceManager::get_feed_id(ETH).unwrap(),
			min_round_id + 1,
			42 * DOLLARS,
		)
		.unwrap();
		ChainlinkFeed::submit(
			Origin::signed(oracle1),
			ChainlinkPriceManager::get_feed_id(KSM).unwrap(),
			min_round_id + 1,
			42 * DOLLARS,
		)
		.unwrap();
		// min_round_id still zero
		assert_eq!(ChainlinkPriceManager::get_min_round_id().unwrap(), 0);
		ChainlinkFeed::submit(
			Origin::signed(oracle1),
			ChainlinkPriceManager::get_feed_id(DOT).unwrap(),
			min_round_id + 1,
			42 * DOLLARS,
		)
		.unwrap();
		assert_eq!(ChainlinkPriceManager::get_min_round_id().unwrap(), 1);
	});
}

#[test]
fn get_feed_id() {
	let oracles_admin: AccountId = 999;
	let oracle1: AccountId = 100;
	test_externalities().execute_with(|| {
		ChainlinkFeed::create_feed(
			alice_origin(),
			20,
			10,
			(10, 1_000 * DOLLARS),
			1,
			5,
			b"MIN-BTC".to_vec(),
			0,
			vec![(oracle1, oracles_admin)],
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
			3, // min submissions
			5,
			b"MIN-BTC".to_vec(),
			2,
			vec![
				(oracle1, oracles_admin),
				(oracle2, oracles_admin),
				(oracle3, oracles_admin),
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
