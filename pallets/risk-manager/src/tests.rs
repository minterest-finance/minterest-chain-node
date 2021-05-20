//! Tests for the risk-manager pallet.
/// Unit tests for liquidation functions see in unit-tests for runtime.
use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	Externalities, OffchainExt, StorageKind, TransactionPoolExt,
};

use frame_support::sp_runtime::app_crypto::sp_core::offchain::{
	HttpError, HttpRequestId, HttpRequestStatus, OpaqueNetworkState, Timestamp,
};
use frame_support::sp_runtime::app_crypto::sp_core::OpaquePeerId;
use liquidation_pools;
use sp_core::offchain::OffchainStorage;

// TODO: Make this more general by taking a struct containing hooks
/// Allows you to hook into timestamp call
struct OffChainExtWithHooks<T> {
	inner: Box<T>,
	timestamp_hook: Option<Box<dyn Fn(Timestamp, &mut dyn Externalities) -> Timestamp + Send>>,
}

impl<T> OffChainExtWithHooks<T>
where
	T: Externalities,
{
	pub fn new(
		inner: T,
		timestamp_hook: Option<Box<dyn Fn(Timestamp, &mut dyn Externalities) -> Timestamp + Send>>,
	) -> Self {
		Self {
			inner: Box::new(inner),
			timestamp_hook,
		}
	}
}

impl<T> Externalities for OffChainExtWithHooks<T>
where
	T: Externalities,
{
	fn is_validator(&self) -> bool {
		self.inner.is_validator()
	}

	fn network_state(&self) -> Result<OpaqueNetworkState, ()> {
		self.inner.network_state()
	}

	fn timestamp(&mut self) -> Timestamp {
		let inner_timestamp = self.inner.timestamp();
		return if let Some(ref mut timestamp_hook_fn) = self.timestamp_hook {
			timestamp_hook_fn(inner_timestamp, self.inner.as_mut())
		} else {
			inner_timestamp
		};
	}

	fn sleep_until(&mut self, deadline: Timestamp) {
		self.inner.sleep_until(deadline)
	}

	fn random_seed(&mut self) -> [u8; 32] {
		self.inner.random_seed()
	}

	fn local_storage_set(&mut self, kind: StorageKind, key: &[u8], value: &[u8]) {
		self.inner.local_storage_set(kind, key, value)
	}

	fn local_storage_clear(&mut self, kind: StorageKind, key: &[u8]) {
		self.inner.local_storage_clear(kind, key)
	}

	fn local_storage_compare_and_set(
		&mut self,
		kind: StorageKind,
		key: &[u8],
		old_value: Option<&[u8]>,
		new_value: &[u8],
	) -> bool {
		self.inner
			.local_storage_compare_and_set(kind, key, old_value, new_value)
	}

	fn local_storage_get(&mut self, kind: StorageKind, key: &[u8]) -> Option<Vec<u8>> {
		self.inner.local_storage_get(kind, key)
	}

	fn http_request_start(&mut self, method: &str, uri: &str, meta: &[u8]) -> Result<HttpRequestId, ()> {
		self.inner.http_request_start(method, uri, meta)
	}

	fn http_request_add_header(&mut self, request_id: HttpRequestId, name: &str, value: &str) -> Result<(), ()> {
		self.inner.http_request_add_header(request_id, name, value)
	}

	fn http_request_write_body(
		&mut self,
		request_id: HttpRequestId,
		chunk: &[u8],
		deadline: Option<Timestamp>,
	) -> Result<(), HttpError> {
		self.inner.http_request_write_body(request_id, chunk, deadline)
	}

	fn http_response_wait(&mut self, ids: &[HttpRequestId], deadline: Option<Timestamp>) -> Vec<HttpRequestStatus> {
		self.inner.http_response_wait(ids, deadline)
	}

	fn http_response_headers(&mut self, request_id: HttpRequestId) -> Vec<(Vec<u8>, Vec<u8>)> {
		self.inner.http_response_headers(request_id)
	}

	fn http_response_read_body(
		&mut self,
		request_id: HttpRequestId,
		buffer: &mut [u8],
		deadline: Option<Timestamp>,
	) -> Result<usize, HttpError> {
		self.inner.http_response_read_body(request_id, buffer, deadline)
	}

	fn set_authorized_nodes(&mut self, nodes: Vec<OpaquePeerId>, authorized_only: bool) {
		self.inner.set_authorized_nodes(nodes, authorized_only)
	}
}

#[test]
fn test_offchain_worker_lock_expired() {
	let mut ext = ExtBuilder::default()
		.pool_init(ETH)
		.pool_init(BTC)
		.user_balance(ALICE, BTC, 100_000 * DOLLARS)
		.liquidity_pool_balance(BTC, 15_000 * DOLLARS)
		.liquidity_pool_balance(ETH, 15_000 * DOLLARS)
		.liquidation_pool_balance(ETH, 10_000 * DOLLARS)
		.liquidation_pool_balance(BTC, 10_000 * DOLLARS)
		.build();

	let (offchain, state) = TestOffchainExt::new();
	let offchain_ext_with_timestamp_hook = OffChainExtWithHooks::new(
		// Increment time by some amount whenever timestamp is called, to simulate real clock
		offchain,
		Some(Box::new(|t, ext| {
			ext.sleep_until(t.add(Duration::from_millis(30000)));
			t
		})),
	);

	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain_ext_with_timestamp_hook));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice(),
			BTC,
			11_000 * DOLLARS
		));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice(), BTC));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::borrow(alice(), ETH, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan shoud be liquidated
		Prices::unlock_price(admin(), BTC).unwrap();

		assert_ok!(TestRiskManager::_offchain_worker());

		// There are two transactions. One of them is liquidation of loan, another one is balancing of pool
		assert_eq!(trans_pool_state.read().transactions.len(), 2);

		// It check is balancing pool extrinsic was called.
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		match ex.call {
			crate::mock::Call::LiquidationPools(liquidation_pools::Call::balance_liquidation_pools(..)) => {}
			e => panic!("Unexpected call: {:?}", e),
		}

		// It check is liquidation extrinsic was called.
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		// Called extrinsic input params
		let (who, pool_id) = match ex.call {
			crate::mock::Call::TestRiskManager(crate::Call::liquidate(who, pool_id, ..)) => (who, pool_id),
			e => panic!("Unexpected call: {:?}", e),
		};

		assert_eq!(who, ALICE);
		assert_eq!(pool_id, ETH);
		// Get saved index from database
		let serialized_index_result = state
			.read()
			.local_storage
			.get(b"", OFFCHAIN_WORKER_LATEST_POOL_INDEX)
			.unwrap();
		// If sequence that produced by CurrencyId::get_enabled_tokens_in_protocol was changed, this
		// assertion can fail.
		assert_eq!(u32::decode(&mut &*serialized_index_result).unwrap(), 3);

		// Shouldn't fail
		assert_ok!(TestRiskManager::_offchain_worker());
	});
}

#[test]
fn test_offchain_worker_simple_liquidation() {
	let mut ext = ExtBuilder::default()
		.pool_init(DOT)
		.pool_init(KSM)
		.user_balance(ALICE, DOT, 100_000 * DOLLARS)
		.liquidity_pool_balance(DOT, 10_000 * DOLLARS)
		.liquidity_pool_balance(KSM, 15_000 * DOLLARS)
		.build();

	let (offchain, _state) = TestOffchainExt::new();
	let (pool, trans_pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		System::set_block_number(2);
		assert_ok!(TestMinterestProtocol::deposit_underlying(
			alice(),
			DOT,
			11_000 * DOLLARS
		));
		assert_ok!(TestMinterestProtocol::enable_is_collateral(alice(), DOT));

		System::set_block_number(3);
		assert_ok!(TestMinterestProtocol::borrow(alice(), KSM, 10_500 * DOLLARS));

		System::set_block_number(4);
		// Decrease DOT price. Now alice collateral isn't enough
		// and loan shoud be liquidated
		Prices::unlock_price(admin(), DOT).unwrap();

		assert_ok!(TestRiskManager::_offchain_worker());

		assert_eq!(trans_pool_state.read().transactions.len(), 1);
		let transaction = trans_pool_state.write().transactions.pop().unwrap();
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();

		// Called extrinsic input params
		let (who, pool_id) = match ex.call {
			crate::mock::Call::TestRiskManager(crate::Call::liquidate(who, pool_id, ..)) => (who, pool_id),
			e => panic!("Unexpected call: {:?}", e),
		};
		assert_eq!(who, ALICE);
		assert_eq!(pool_id, KSM);
	});
}

#[test]
fn set_max_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), DOT, 0));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).max_attempts, 0);
		let expected_event = Event::risk_manager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(0));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set max_attempts equal 2.0
		assert_ok!(TestRiskManager::set_max_attempts(admin(), DOT, 2));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).max_attempts, 2);
		let expected_event = Event::risk_manager(crate::Event::MaxValueOFLiquidationAttempsHasChanged(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_max_attempts(alice(), DOT, 10), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_max_attempts(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_min_partial_liquidation_sum_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin(),
			DOT,
			Balance::zero()
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(DOT).min_partial_liquidation_sum,
			Balance::zero()
		);
		let expected_event = Event::risk_manager(crate::Event::MinSumForPartialLiquidationHasChanged(Balance::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal to one hundred.
		assert_ok!(TestRiskManager::set_min_partial_liquidation_sum(
			admin(),
			DOT,
			ONE_HUNDRED * DOLLARS
		));
		assert_eq!(
			TestRiskManager::risk_manager_dates(DOT).min_partial_liquidation_sum,
			ONE_HUNDRED * DOLLARS
		);
		let expected_event = Event::risk_manager(crate::Event::MinSumForPartialLiquidationHasChanged(
			ONE_HUNDRED * DOLLARS,
		));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(alice(), DOT, 10),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_min_partial_liquidation_sum(admin(), MDOT, 10),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_threshold_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 0.0
		assert_ok!(TestRiskManager::set_threshold(admin(), DOT, Rate::zero()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).threshold, Rate::zero());
		let expected_event = Event::risk_manager(crate::Event::ValueOfThresholdHasChanged(Rate::zero()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// ALICE set min_partial_liquidation_sum equal one hundred.
		assert_ok!(TestRiskManager::set_threshold(admin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).threshold, Rate::one());
		let expected_event = Event::risk_manager(crate::Event::ValueOfThresholdHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// The dispatch origin of this call must be Administrator.
		assert_noop!(TestRiskManager::set_threshold(alice(), DOT, Rate::one()), BadOrigin);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_threshold(admin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn set_liquidation_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Can be set to 1.0
		assert_ok!(TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::one()));
		assert_eq!(TestRiskManager::risk_manager_dates(DOT).liquidation_fee, Rate::one());
		let expected_event = Event::risk_manager(crate::Event::ValueOfLiquidationFeeHasChanged(Rate::one()));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Can not be set to 0.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::zero()),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// Can not be set to 2.0
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), DOT, Rate::saturating_from_integer(2)),
			Error::<Test>::InvalidLiquidationIncentiveValue
		);

		// The dispatch origin of this call must be Administrator.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(alice(), DOT, Rate::one()),
			BadOrigin
		);

		// MDOT is wrong CurrencyId for underlying assets.
		assert_noop!(
			TestRiskManager::set_liquidation_fee(admin(), MDOT, Rate::one()),
			Error::<Test>::NotValidUnderlyingAssetId
		);
	});
}

#[test]
fn liquidate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// Origin::signed(Alice) is wrong origin for fn liquidate.
		assert_noop!(TestRiskManager::liquidate(Origin::signed(ALICE), ALICE, DOT), BadOrigin);

		// Origin::none is available origin for fn liquidate.
		assert_noop!(
			TestRiskManager::liquidate(Origin::none(), ALICE, DOT),
			minterest_protocol::Error::<Test>::ZeroBalanceTransaction
		);
	})
}

#[test]
fn mutate_liquidation_attempts_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, true);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			u8::one()
		);
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, true);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			2_u8
		);
		TestRiskManager::mutate_liquidation_attempts(DOT, &ALICE, false);
		assert_eq!(
			liquidity_pools::PoolUserParams::<Test>::get(DOT, ALICE).liquidation_attempts,
			u8::zero()
		);
	})
}
