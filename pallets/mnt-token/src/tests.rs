#![cfg(test)]

use super::Error;
use crate::mock::*;
use frame_support::pallet_prelude::Hooks;
use frame_support::{assert_noop, assert_ok};
use minterest_primitives::{Balance, CurrencyId, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::MntManager;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::Zero;

const MNT_PALLET_START_BALANCE: Balance = 1_000_000 * DOLLARS;

fn get_mnt_account_balance(user: AccountId) -> Balance {
	Currencies::free_balance(MNT, &user)
}

fn run_to_block(n: u64) {
	while System::block_number() < n {
		MntToken::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
	}
}

/// Move flywheel and check borrower balance
fn check_borrower(
	pool_id: CurrencyId,
	borrower: AccountId,
	expected_mnt_balance: Balance,
	expected_mnt_in_storage: Balance,
) {
	assert_ok!(MntToken::update_mnt_borrow_index(pool_id));
	assert_ok!(MntToken::distribute_borrower_mnt(pool_id, &borrower, false));

	let pool_state = MntToken::mnt_pools_state(pool_id).borrow_state;
	let borrower_index = MntToken::mnt_borrower_index(pool_id, borrower);
	assert_eq!(borrower_index, pool_state.mnt_distribution_index);

	assert_eq!(get_mnt_account_balance(borrower), expected_mnt_balance);
	assert_eq!(MntToken::mnt_accrued(borrower), expected_mnt_in_storage);
}

/// Move flywheel and check supplier balance
fn check_supplier_accrued(
	pool_id: CurrencyId,
	supplier: AccountId,
	expected_mnt_balance: Balance,
	expected_mnt_in_storage: Balance,
) {
	assert_ok!(MntToken::update_mnt_supply_index(pool_id));
	assert_ok!(MntToken::distribute_supplier_mnt(pool_id, &supplier, false));
	assert_eq!(get_mnt_account_balance(supplier), expected_mnt_balance);
	assert_eq!(MntToken::mnt_accrued(supplier), expected_mnt_in_storage);
}

#[test]
fn distribute_mnt_to_borrower_with_threshold() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 150_000 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(20)
		.pool_user_data(
			DOT,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5 too
			true,
			0,
		)
		.build()
		.execute_with(|| {
			// Award for ALICE is 10 per block
			// So at the first step awarded tokens should be kept in internal storage
			// At the second it should be transferred to ALICE and so on.

			let mnt_rate = 10 * DOLLARS;
			assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));
			// First interaction with protocol for distributors.
			// This is a starting point to earn MNT token
			assert_ok!(MntToken::update_mnt_borrow_index(DOT));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));
			check_borrower(DOT, ALICE, 0, 0);

			System::set_block_number(2);
			// 10 tokens in internal storage
			check_borrower(DOT, ALICE, 0, mnt_rate);

			System::set_block_number(3);
			// 20 tokens on account balance
			check_borrower(DOT, ALICE, mnt_rate * 2, 0);

			System::set_block_number(4);
			// 10 tokens in internal storage and 20 on account balance
			check_borrower(DOT, ALICE, mnt_rate * 2, mnt_rate);

			System::set_block_number(5);
			// 40 tokens on account balance
			check_borrower(DOT, ALICE, mnt_rate * 4, 0);

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_mnt_to_supplier_with_threshold() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(20)
		.pool_total_borrowed(DOT, 100 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			// Award for ALICE is 10 per block
			// So at the first step awarded tokens should be kept in internal storage
			// At the second it should be transferred to ALICE and so on.

			// set total issuance
			Currencies::deposit(MDOT, &ALICE, 100 * DOLLARS).unwrap();

			check_supplier_accrued(DOT, ALICE, 0, 10 * DOLLARS);
			System::set_block_number(2);
			check_supplier_accrued(DOT, ALICE, 20 * DOLLARS, 0);
			System::set_block_number(3);
			check_supplier_accrued(DOT, ALICE, 20 * DOLLARS, 10 * DOLLARS);
			System::set_block_number(4);
			check_supplier_accrued(DOT, ALICE, 40 * DOLLARS, 0);
			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_mnt_to_supplier_from_different_pools() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_total_borrowed(DOT, 100 * DOLLARS)
		.pool_total_borrowed(KSM, 100 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			// Check accruing mnt tokens from two pools for supplier
			let dot_mnt_speed = 2 * DOLLARS;
			let ksm_mnt_speed = 8 * DOLLARS;
			assert_eq!(MntToken::mnt_speeds(DOT), dot_mnt_speed);
			assert_eq!(MntToken::mnt_speeds(KSM), ksm_mnt_speed);

			// set total issuance
			Currencies::deposit(MDOT, &ALICE, 100 * DOLLARS).unwrap();
			Currencies::deposit(MKSM, &ALICE, 100 * DOLLARS).unwrap();

			check_supplier_accrued(KSM, ALICE, ksm_mnt_speed, 0);
			check_supplier_accrued(DOT, ALICE, ksm_mnt_speed + dot_mnt_speed, 0);
			// The Block number wasn't changed, so we should get the same result without errors
			check_supplier_accrued(DOT, ALICE, ksm_mnt_speed + dot_mnt_speed, 0);

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_mnt_to_borrower_from_different_pools() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 150_000 * DOLLARS)
		.pool_total_borrowed(KSM, 150_000 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_user_data(
			DOT,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
			0,
		)
		.pool_user_data(
			KSM,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
			0,
		)
		.build()
		.execute_with(|| {
			// Check accruing mnt tokens from two pools for borrower
			let mnt_rate = 10 * DOLLARS;
			assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));

			// First interaction with protocol for distributors.
			// This is a starting point to earn MNT token
			assert_ok!(MntToken::update_mnt_borrow_index(DOT));
			assert_ok!(MntToken::update_mnt_borrow_index(KSM));
			assert_ok!(MntToken::distribute_borrower_mnt(KSM, &ALICE, false));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));

			System::set_block_number(2);

			// Move flywheel
			assert_ok!(MntToken::update_mnt_borrow_index(DOT));
			assert_ok!(MntToken::update_mnt_borrow_index(KSM));
			assert_ok!(MntToken::distribute_borrower_mnt(KSM, &ALICE, false));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));

			assert_eq!(get_mnt_account_balance(ALICE), mnt_rate);

			let dot_mnt_speed = 2 * DOLLARS;
			// Check event about distributing mnt tokens by DOT pool
			let borrower_index = MntToken::mnt_borrower_index(DOT, ALICE);
			let event = Event::mnt_token(crate::Event::MntDistributedToBorrower(
				DOT,
				ALICE,
				dot_mnt_speed,
				borrower_index,
			));
			assert!(System::events().iter().any(|record| record.event == event));

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_borrowers_mnt() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_total_borrowed(DOT, 150_000 * DOLLARS)
		.pool_user_data(
			DOT,
			ALICE,
			30_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
			0,
		)
		.pool_user_data(
			DOT,
			BOB,
			120_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
			0,
		)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			/*
			There is only one pool included in minting process. So 10 mnt for this pool.
			Pool total borrow is 150_000. Alice borrowed 30_000 and BOB - 120_000

			This is a part of liquidity which belongs to Alice.
			30 / 150 = 0.2.

			10(mnt per block) * 0.2(alice part) = 2.
			This is how many MNT tokens per block Alice should acquire as a borrower.

			For Bob: 120 / 150 = 0.8; 0.8 * 10 = 8

			First interaction with protocol for distributors.
			This is started point to earn MNT token
			 */
			assert_ok!(MntToken::update_mnt_borrow_index(DOT));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &BOB, false));

			System::set_block_number(2);
			check_borrower(DOT, ALICE, 2 * DOLLARS, 0);
			check_borrower(DOT, BOB, 8 * DOLLARS, 0);

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE) - get_mnt_account_balance(BOB),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_borrower_mnt() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 150_000 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_user_data(
			DOT,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5 too
			true,
			0,
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				MNT_PALLET_START_BALANCE,
				get_mnt_account_balance(MntToken::get_account_id())
			);
			let mnt_rate = 12 * DOLLARS;
			assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));
			// First interaction with protocol for distributors.
			// This is a starting point to earn MNT token
			assert_ok!(MntToken::update_mnt_borrow_index(DOT));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));

			System::set_block_number(2);
			// Alice account borrow balance is 150_000
			check_borrower(DOT, ALICE, mnt_rate, 0);

			// block_delta == 2
			System::set_block_number(4);
			check_borrower(DOT, ALICE, mnt_rate * 3, 0);
			// check twice, move flywheel again
			check_borrower(DOT, ALICE, mnt_rate * 3, 0);

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn test_update_mnt_borrow_index() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 10_000 * DOLLARS)
		.pool_total_borrowed(ETH, 20_000 * DOLLARS)
		.pool_total_borrowed(KSM, 30_000 * DOLLARS)
		.pool_total_borrowed(BTC, 40_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let initial_index = Rate::saturating_from_integer(1);
			let mnt_rate = 1 * DOLLARS;
			assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));
			// Input parameters:
			// mnt_rate: 1
			// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
			// utilities: DOT = $5000, ETH = $30000, KSM = $60000, BTC = $120000
			// sum_of_all_pools_utilities = 215000

			let (currency_utilities, total_utility): (Vec<(CurrencyId, Balance)>, Balance) =
				MntToken::calculate_enabled_pools_utilities().unwrap();
			assert!(currency_utilities.contains(&(DOT, 5000 * DOLLARS)));
			assert!(currency_utilities.contains(&(ETH, 30000 * DOLLARS)));
			assert!(currency_utilities.contains(&(KSM, 60000 * DOLLARS)));
			assert!(currency_utilities.contains(&(BTC, 120000 * DOLLARS)));
			assert_eq!(total_utility, 215000 * DOLLARS);

			// Check mnt speeds
			// 0.0232558139534883721
			let dot_mnt_speed = 23255813953488372;
			// 0.139534883720930233
			let eth_mnt_speed = 139534883720930233;
			// 0.279069767441860465
			let ksm_mnt_speed = 279069767441860465;
			// 0.558139534883720930
			let btc_mnt_speed = 558139534883720930;
			assert_eq!(MntToken::mnt_speeds(DOT), dot_mnt_speed);
			assert_eq!(MntToken::mnt_speeds(BTC), btc_mnt_speed);
			assert_eq!(MntToken::mnt_speeds(ETH), eth_mnt_speed);
			assert_eq!(MntToken::mnt_speeds(KSM), ksm_mnt_speed);
			assert_eq!(dot_mnt_speed + eth_mnt_speed + ksm_mnt_speed + btc_mnt_speed, mnt_rate);

			System::set_block_number(2);

			let check_borrow_index = |underlying_id: CurrencyId, pool_mnt_speed: Balance, total_borrow: Balance| {
				MntToken::update_mnt_borrow_index(underlying_id).unwrap();
				// 1.5 current borrow_index. I use 15 in this function, that why I make total_borrow * 10
				let borrow_total_amount = Rate::saturating_from_rational(total_borrow * 10, 15);

				let expected_index = initial_index + Rate::from_inner(pool_mnt_speed) / borrow_total_amount;
				let pool_state = MntToken::mnt_pools_state(underlying_id);
				assert_eq!(pool_state.borrow_state.mnt_distribution_index, expected_index);
			};

			check_borrow_index(DOT, dot_mnt_speed, 10_000);
			check_borrow_index(ETH, eth_mnt_speed, 20_000);
			check_borrow_index(KSM, ksm_mnt_speed, 30_000);
			check_borrow_index(BTC, btc_mnt_speed, 40_000);
		});
}

#[test]
fn test_update_mnt_borrow_index_simple() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		// total borrows needs to calculate mnt_speeds
		.pool_total_borrowed(DOT, 150_000 * DOLLARS)
		.set_mnt_rate(1)
		.build()
		.execute_with(|| {
			/*
			* Minting was enabled when block_number was equal to 0. Here block_number == 1.
			So block_delta = 1

			Input parameters: 	mnt_speed = 1,
								total_borrowed = 150,
								   pool_borrow_index = 1.5,
								   mnt_acquired = delta_blocks * mnt_speed = 1

			This is how much currency was borrowed without interest
			borrow_total_amount = total_borrowed(150000) / pool_borrow_index(1.5)  = 100000

			How much MNT tokens were earned per block
			ratio = mnt_acquired / borrow_total_amount = 0.00001

			mnt_borrow_index = mnt_borrow_index(1 as initial value) + ratio(0.00001) = 1.00001

			*ratio is amount of MNT tokens for 1 borrowed token
			*/

			MntToken::update_mnt_borrow_index(DOT).unwrap();
			let pool_state = MntToken::mnt_pools_state(DOT);
			assert_eq!(
				pool_state.borrow_state.mnt_distribution_index,
				Rate::saturating_from_rational(100001, 100000)
			);
		});
}

#[test]
fn test_distribute_mnt_tokens_to_suppliers() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		// total borrows needs to calculate mnt_speeds
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			/*
			Minting was enabled when block_number was equal to 0. Here block_number == 1.
			So block_delta = 1

			Input parameters: 10 mnt for suppliers per block.

			There is only one pool included in minting process. So 10 mnt for this pool.
			Total issuance is 100. Alice has 20 MDOT and BOB 80 MDOT.

			This is part from whole circulated wrapped currency held by Alice.
			20 / 100 = 0.2.

			10(mnt per block) * 0.2(alice part) = 2.
			This is how many Alice should acquire MNT tokens per block as supplier.

			For Bob: 80 / 100 = 0.8; 0.8 * 10 = 8
			 */
			let alice_balance = 20 * DOLLARS;
			let bob_balance = 80 * DOLLARS;
			let alice_award_per_block = 2 * DOLLARS;
			let bob_award_per_block = 8 * DOLLARS;

			// set total issuance
			Currencies::deposit(MDOT, &ALICE, alice_balance).unwrap();
			Currencies::deposit(MDOT, &BOB, bob_balance).unwrap();

			let move_flywheel = || {
				MntToken::update_mnt_supply_index(DOT).unwrap();
				MntToken::distribute_supplier_mnt(DOT, &ALICE, false).unwrap();
				MntToken::distribute_supplier_mnt(DOT, &BOB, false).unwrap();
			};

			let check_supplier_award =
				|supplier_id: AccountId, distributed_amount: Balance, expected_user_mnt_balance: Balance| {
					let pool_state = MntToken::mnt_pools_state(DOT);
					let supplier_index = MntToken::mnt_supplier_index(DOT, supplier_id).unwrap();
					assert_eq!(supplier_index, pool_state.supply_state.mnt_distribution_index);
					assert_eq!(get_mnt_account_balance(supplier_id), expected_user_mnt_balance);
					// it should be 0 because threshold is 0
					assert_eq!(MntToken::mnt_accrued(supplier_id), 0);

					let supplier_index = MntToken::mnt_supplier_index(DOT, supplier_id).unwrap();
					let event = Event::mnt_token(crate::Event::MntDistributedToSupplier(
						DOT,
						supplier_id,
						distributed_amount,
						supplier_index,
					));
					assert!(System::events().iter().any(|record| record.event == event));
				};

			/* -------TEST SCENARIO------- */
			move_flywheel();
			check_supplier_award(ALICE, alice_award_per_block, alice_award_per_block);
			check_supplier_award(BOB, bob_award_per_block, bob_award_per_block);

			// Go from first block to third
			System::set_block_number(3);
			let current_block = 3;
			let block_delta = 2;
			move_flywheel();
			check_supplier_award(
				BOB,
				bob_award_per_block * block_delta,
				bob_award_per_block * current_block,
			);
			check_supplier_award(
				BOB,
				bob_award_per_block * block_delta,
				bob_award_per_block * current_block,
			);
			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE) - get_mnt_account_balance(BOB),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn test_update_mnt_supply_index() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		// total borrows needs to calculate mnt_speeds
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			//
			// * Minting was enabled when block_number was equal to 0. Here block_number == 1.
			// So block_delta = 1
			//

			// set total issuance
			let mdot_total_issuance = 10 * DOLLARS;
			let meth_total_issuance = 20 * DOLLARS;
			let mksm_total_issuance = 30 * DOLLARS;
			let mbtc_total_issuance = 40 * DOLLARS;
			Currencies::deposit(MDOT, &ALICE, mdot_total_issuance).unwrap();
			Currencies::deposit(METH, &ALICE, meth_total_issuance).unwrap();
			Currencies::deposit(MKSM, &ALICE, mksm_total_issuance).unwrap();
			Currencies::deposit(MBTC, &ALICE, mbtc_total_issuance).unwrap();

			let dot_mnt_speed = 714285714285714280;
			assert_eq!(MntToken::mnt_speeds(DOT), dot_mnt_speed);
			let ksm_mnt_speed = 2857142857142857140;
			assert_eq!(MntToken::mnt_speeds(KSM), ksm_mnt_speed);
			let eth_mnt_speed = 2142857142857142850;
			assert_eq!(MntToken::mnt_speeds(ETH), eth_mnt_speed);
			let btc_mnt_speed = 4285714285714285710;
			assert_eq!(MntToken::mnt_speeds(BTC), btc_mnt_speed);

			let check_supply_index = |underlying_id: CurrencyId, mnt_speed: Balance, total_issuance: Balance| {
				MntToken::update_mnt_supply_index(underlying_id).unwrap();
				let pool_state = MntToken::mnt_pools_state(underlying_id);
				assert_eq!(
					pool_state.supply_state.mnt_distribution_index,
					Rate::one() + Rate::from_inner(mnt_speed) / Rate::from_inner(total_issuance)
				);
				assert_eq!(pool_state.supply_state.index_updated_at_block, 1);
			};
			check_supply_index(DOT, dot_mnt_speed, mdot_total_issuance);
			check_supply_index(KSM, ksm_mnt_speed, mksm_total_issuance);
			check_supply_index(ETH, eth_mnt_speed, meth_total_issuance);
			check_supply_index(BTC, btc_mnt_speed, mbtc_total_issuance);
		});
}

#[test]
fn test_update_mnt_supply_index_simple() {
	ExtBuilder::default()
		// total_borrow shouldn't be zero at least for one market to calculate mnt speeds
		.pool_total_borrowed(ETH, 150_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Input parameters:
			// supply_state.block_number = 1, supply_state.index = 1,
			// mnt_speed = 10, total_supply = 20
			// *mnt_speed = mnt_rate because the only one pool is included

			// set total_issuance to 20
			Currencies::deposit(METH, &ALICE, 20 * DOLLARS).unwrap();
			let mnt_rate = 10 * DOLLARS;
			assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));
			assert_ok!(MntToken::enable_mnt_minting(admin(), ETH));

			System::set_block_number(2);
			MntToken::update_mnt_supply_index(ETH).unwrap();
			let pool_state = MntToken::mnt_pools_state(ETH);
			// block_delta = current_block(2) - supply_state.block_number(1) = 1
			// mnt_accrued = block_delta(1) * mnt_speed(10) = 10
			// ratio = mnt_accrued(10) / total_supply(20) = 0.5
			// supply_state.index = supply_state.index(1) + ratio(0.5) = 1.5
			// supply_state.block_number = current_block = 2
			assert_eq!(
				pool_state.supply_state.mnt_distribution_index,
				Rate::saturating_from_rational(15, 10)
			);
			assert_eq!(pool_state.supply_state.index_updated_at_block, 2);
		});
}

#[test]
fn test_mnt_speed_calculation() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.build()
		.execute_with(|| {
			let mnt_rate = 10 * DOLLARS;
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
			let expected_dot_mnt_speed = 714285714285714280;
			assert_eq!(MntToken::mnt_speeds(DOT), expected_dot_mnt_speed);

			// KSM
			// utility_ftaction = 100 / 350 = 0.285714285714285714
			// mnt_speed = utility_fraction * mnt_rate = 2.85714285714285714
			let expected_ksm_mnt_speed = 2857142857142857140;
			assert_eq!(MntToken::mnt_speeds(KSM), expected_ksm_mnt_speed);

			// ETH
			// utility_ftaction = 75 / 350 = 0.214285714285714285
			// mnt_speed = utility_fraction * mnt_rate = 2.14285714285714285
			let expected_eth_mnt_speed = 2142857142857142850;
			assert_eq!(MntToken::mnt_speeds(ETH), expected_eth_mnt_speed);

			// BTC
			// utility_ftaction = 150 / 350 = 0.428571428571428571
			// mnt_speed = utility_fraction * mnt_rate = 4.28571428571428571
			let expected_btc_mnt_speed = 4285714285714285710;
			assert_eq!(MntToken::mnt_speeds(BTC), expected_btc_mnt_speed);

			// Sum of all mnt_speeds is equal to mnt_rate
			let sum = expected_dot_mnt_speed + expected_btc_mnt_speed + expected_eth_mnt_speed + expected_ksm_mnt_speed;
			assert_eq!(Rate::from_inner(sum).round().into_inner(), mnt_rate);

			// Multiply mnt rate in twice. Expected mnt speeds should double up
			let mnt_rate = mnt_rate * 2;
			assert_ok!(MntToken::set_mnt_rate(admin(), mnt_rate));
			assert_eq!(MntToken::mnt_speeds(DOT), expected_dot_mnt_speed * 2);
			assert_eq!(MntToken::mnt_speeds(KSM), expected_ksm_mnt_speed * 2);
			assert_eq!(MntToken::mnt_speeds(ETH), expected_eth_mnt_speed * 2);
			assert_eq!(MntToken::mnt_speeds(BTC), expected_btc_mnt_speed * 2);
		});
}

#[test]
fn test_mnt_speed_calculation_with_zero_borrowed() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 0 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			// Input parameters:
			// mnt_rate: 10
			// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
			// utilities: DOT = 25, ETH = 0, KSM = 100, BTC = 150
			// sum_of_all_pools_utilities = 275
			let (currency_utilities, total_utility) = MntToken::calculate_enabled_pools_utilities().unwrap();
			assert!(currency_utilities.contains(&(DOT, 25 * DOLLARS)));
			assert!(currency_utilities.contains(&(ETH, 0 * DOLLARS)));
			assert!(currency_utilities.contains(&(KSM, 100 * DOLLARS)));
			assert!(currency_utilities.contains(&(BTC, 150 * DOLLARS)));
			assert_eq!(total_utility, 275 * DOLLARS);

			// MntSpeed for ETH is 0 because total_borrowed is 0
			assert_eq!(MntToken::mnt_speeds(ETH), Balance::zero());

			// DOT
			// utility_fraction = 25 / 275 = 0.071428571428571428
			// mnt_speed = utility_fraction * mnt_rate = 0.909090909090909090
			assert_eq!(MntToken::mnt_speeds(DOT), 909090909090909090);

			// KSM
			// utility_ftaction = 100 / 275 = 0.363636363636363636
			// mnt_speed = utility_fraction * mnt_rate = 3.636363636363636360
			assert_eq!(MntToken::mnt_speeds(KSM), 3636363636363636360);

			// BTC
			// utility_ftaction = 150 / 275 = 0.545454545454545454
			// mnt_speed = utility_fraction * mnt_rate = 5.454545454545454540
			assert_eq!(MntToken::mnt_speeds(BTC), 5454545454545454540);
		});
}

#[test]
fn test_disable_mnt_minting() {
	// 1. Disable MNT minting for one pool and check mnt speeds recalculation
	// 2. Enable MNT minting for disabled pool
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			// Make sure that speeds were precalculated
			assert_eq!(MntToken::mnt_speeds(DOT), 714285714285714280);

			// Disable MNT minting for BTC
			assert_ok!(MntToken::disable_mnt_minting(admin(), BTC));
			assert_eq!(MntToken::mnt_speeds(BTC), Balance::zero());
			// Now utilities: DOT = 25, ETH = 75, KSM = 100
			// sum_of_all_pools_utilities = 200

			// DOT mnt_speed = 25 / 200 * 10 = 1.25
			let expected_dot_mnt_speed = Rate::saturating_from_rational(125, 100).into_inner();
			assert_eq!(MntToken::mnt_speeds(DOT), expected_dot_mnt_speed);

			// ETH mnt_speed 75 / 200 * 10 = 3.75
			let expected_eth_mnt_speed = Rate::saturating_from_rational(375, 100).into_inner();
			assert_eq!(MntToken::mnt_speeds(ETH), expected_eth_mnt_speed);

			// KSM mnt_speed = 100 / 200 * 10 = 5
			assert_eq!(MntToken::mnt_speeds(KSM), 5 * DOLLARS);

			// Enable MNT minting for BTC.
			// MNT speeds should be recalculated again
			assert_ok!(MntToken::enable_mnt_minting(admin(), BTC));
			assert_eq!(MntToken::mnt_speeds(DOT), 714285714285714280);
			assert_eq!(MntToken::mnt_speeds(KSM), 2857142857142857140);
			assert_eq!(MntToken::mnt_speeds(ETH), 2142857142857142850);
			assert_eq!(MntToken::mnt_speeds(BTC), 4285714285714285710);
		});
}

#[test]
fn test_disable_generating_all_mnt_tokens() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.build()
		.execute_with(|| {
			assert_ok!(MntToken::set_mnt_rate(admin(), Balance::zero()));
			let zero = Balance::zero();
			assert_eq!(MntToken::mnt_speeds(DOT), zero);
			assert_eq!(MntToken::mnt_speeds(KSM), zero);
			assert_eq!(MntToken::mnt_speeds(ETH), zero);
			assert_eq!(MntToken::mnt_speeds(BTC), zero);
		});
}

#[test]
fn test_set_mnt_rate() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.build()
		.execute_with(|| {
			let test = |new_rate: Balance| {
				let old_rate = MntToken::mnt_rate();
				assert_eq!(MntToken::mnt_rate(), old_rate);
				assert_ok!(MntToken::set_mnt_rate(admin(), new_rate));
				assert_eq!(MntToken::mnt_rate(), new_rate);
				let new_mnt_rate_event = Event::mnt_token(crate::Event::NewMntRate(old_rate, new_rate));
				assert!(System::events().iter().any(|record| record.event == new_mnt_rate_event));
			};

			test(10 * DOLLARS);
			test(15 * DOLLARS);
		});
}

#[test]
fn test_minting_enable_disable() {
	ExtBuilder::default()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			// Add new mnt minting
			assert_ok!(MntToken::enable_mnt_minting(admin(), DOT));
			let new_minting_event = Event::mnt_token(crate::Event::MntMintingEnabled(DOT));
			assert!(System::events().iter().any(|record| record.event == new_minting_event));
			assert_ne!(MntToken::mnt_speeds(DOT), Balance::zero());
			// Try to add the same pool
			assert_noop!(
				MntToken::enable_mnt_minting(admin(), DOT),
				Error::<Runtime>::MntMintingAlreadyEnabled
			);

			// Add minting for another one pool
			assert_ok!(MntToken::enable_mnt_minting(admin(), KSM));
			let new_minting_event = Event::mnt_token(crate::Event::MntMintingEnabled(KSM));
			assert!(System::events().iter().any(|record| record.event == new_minting_event));
			assert_ne!(MntToken::mnt_speeds(KSM), Balance::zero());

			// Disable MNT minting for DOT
			assert_ok!(MntToken::disable_mnt_minting(admin(), DOT));
			let disable_mnt_minting_event = Event::mnt_token(crate::Event::MntMintingDisabled(DOT));
			assert!(System::events()
				.iter()
				.any(|record| record.event == disable_mnt_minting_event));
			assert_eq!(MntToken::mnt_speeds(DOT), Balance::zero());

			// Try to disable minting that wasn't enabled
			assert_noop!(
				MntToken::disable_mnt_minting(admin(), DOT),
				Error::<Runtime>::MntMintingNotEnabled,
			);
		});
}

#[test]
fn test_calculate_enabled_pools_utilities() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.build()
		.execute_with(|| {
			// Amount tokens: 50 for each currency
			// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
			// Expected utilities results: DOT = 25, ETH = 75, KSM = 100, BTC = 150
			let (currency_utilities, total_utility) = MntToken::calculate_enabled_pools_utilities().unwrap();
			assert!(currency_utilities.contains(&(DOT, 25 * DOLLARS)));
			assert!(currency_utilities.contains(&(ETH, 75 * DOLLARS)));
			assert!(currency_utilities.contains(&(KSM, 100 * DOLLARS)));
			assert!(currency_utilities.contains(&(BTC, 150 * DOLLARS)));
			assert_eq!(total_utility, 350 * DOLLARS);
		});
}

#[test]
fn test_calculate_enabled_pools_utilities_fail() {
	ExtBuilder::default()
		.mnt_account_balance(100_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let non_existent_liquidity_pool = MNT;
			assert_noop!(
				MntToken::enable_mnt_minting(admin(), non_existent_liquidity_pool),
				Error::<Runtime>::NotValidUnderlyingAssetId
			);
		});
}

#[test]
fn transfer_mnt_should_work() {
	ExtBuilder::default()
		.set_mnt_claim_threshold(20)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.build()
		.execute_with(|| {
			// distribute_all == false, user_accrued < threshold:
			// we do not perform the transfer.
			let first_transfer = 10 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, first_transfer, false));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE
			);
			assert_eq!(get_mnt_account_balance(ALICE), Balance::zero());
			assert_eq!(MntToken::mnt_accrued(ALICE), first_transfer);

			// distribute_all == true, user_accrued > threshold:
			// we perform the transfer.
			let second_transfer = 200 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, second_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer);
			assert_eq!(MntToken::mnt_accrued(ALICE), Balance::zero());

			// distribute_all == true, user_accrued == 0:
			// we do not perform the transfer.
			let third_transfer = Balance::zero();
			assert_ok!(MntToken::transfer_mnt(&ALICE, third_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer);
			assert_eq!(MntToken::mnt_accrued(ALICE), Balance::zero());

			// distribute_all == true, user_accrued > threshold, user_accrued > MNT_pallet_balance:
			// we do not perform the transfer.
			let fourth_transfer = 10_000_000 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, fourth_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer);
			assert_eq!(MntToken::mnt_accrued(ALICE), Balance::zero());

			// distribute_all == true, user_accrued < threshold:
			// we perform the transfer.
			let fifth_transfer = 10 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, first_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer - fifth_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer + fifth_transfer);
			assert_eq!(MntToken::mnt_accrued(ALICE), Balance::zero());

			// distribute_all == false, user_accrued > threshold:
			// we perform the transfer.
			let sixth_transfer = 500 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, sixth_transfer, false));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer - fifth_transfer - sixth_transfer
			);
			assert_eq!(
				get_mnt_account_balance(ALICE),
				second_transfer + fifth_transfer + sixth_transfer
			);
			assert_eq!(MntToken::mnt_accrued(ALICE), Balance::zero());
		});
}

#[test]
fn on_finalize_should_work() {
	ExtBuilder::default()
		.enable_minting_for_all_pools()
		.set_mnt_claim_threshold(20)
		.pool_total_borrowed(DOT, 50 * DOLLARS)
		.pool_total_borrowed(ETH, 50 * DOLLARS)
		.pool_total_borrowed(KSM, 50 * DOLLARS)
		.pool_total_borrowed(BTC, 50 * DOLLARS)
		.set_mnt_rate(10)
		.build()
		.execute_with(|| {
			// Prices: DOT[0] = 0.5 USD, ETH[1] = 1.5 USD, KSM[2] = 2 USD, BTC[3] = 3 USD
			// Sum of all utilities: 350$
			// Expected speed = pool_utilities / sum_of_all_utilities * MntRate
			// DOT: 25/350 * 10 = 0,714285
			// ETH: 75/350 * 10 = 2,142857
			// KSM: 100/350 * 10 = 2,857142
			// BTC: 150/350 * 10 = 4,285714
			run_to_block(6);
			assert_eq!(MntToken::mnt_speeds(DOT), 714_285_714_285_714_280);
			assert_eq!(MntToken::mnt_speeds(ETH), 2_142_857_142_857_142_850);
			assert_eq!(MntToken::mnt_speeds(KSM), 2_857_142_857_142_857_140);
			assert_eq!(MntToken::mnt_speeds(BTC), 4_285_714_285_714_285_710);

			// Prepare data to see MntSpeed changing
			TestPools::set_pool_data(DOT, 120, Rate::one(), 0).unwrap();
			TestPools::set_pool_data(ETH, 40, Rate::one(), 0).unwrap();
			TestPools::set_pool_data(KSM, 30, Rate::one(), 0).unwrap();
			TestPools::set_pool_data(BTC, 20, Rate::one(), 0).unwrap();

			// Check that nothing changed.
			run_to_block(7);
			assert_eq!(MntToken::mnt_speeds(DOT), 714_285_714_285_714_280);
			assert_eq!(MntToken::mnt_speeds(ETH), 2_142_857_142_857_142_850);
			assert_eq!(MntToken::mnt_speeds(KSM), 2_857_142_857_142_857_140);
			assert_eq!(MntToken::mnt_speeds(BTC), 4_285_714_285_714_285_710);

			// Sum of all utilities: 240$
			// Expected speeds:
			// DOT: 60/240 * 10 = 2.5
			// ETH: 60/240 * 10 = 2.5
			// KSM: 60/240 * 10 = 2.5
			// BTC: 60/240 * 10 = 2.5
			run_to_block(11);
			assert_eq!(MntToken::mnt_speeds(DOT), 2_500_000_000_000_000_000);
			assert_eq!(MntToken::mnt_speeds(ETH), 2_500_000_000_000_000_000);
			assert_eq!(MntToken::mnt_speeds(KSM), 2_500_000_000_000_000_000);
			assert_eq!(MntToken::mnt_speeds(BTC), 2_500_000_000_000_000_000);
		});
}
