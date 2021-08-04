#![cfg(test)]

use super::Error;
use crate::mock::*;
use crate::{MntPoolState, MntState};
use frame_support::{assert_noop, assert_ok};
use minterest_primitives::{Balance, Rate};
use orml_traits::MultiCurrency;
use pallet_traits::MntManager;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::{
	traits::{One, Zero},
	DispatchError::BadOrigin,
};

const MNT_PALLET_START_BALANCE: Balance = 1_000_000 * DOLLARS;

fn get_mnt_account_balance(user: AccountId) -> Balance {
	Currencies::free_balance(MNT_CUR, &user)
}

/// Move flywheel and check borrower balance
fn check_borrower(
	pool_id: OriginalAsset,
	borrower: AccountId,
	expected_mnt_balance: Balance,
	expected_mnt_in_storage: Balance,
) {
	assert_ok!(MntToken::update_pool_mnt_borrow_index(pool_id));
	assert_ok!(MntToken::distribute_borrower_mnt(pool_id, &borrower, false));

	let pool_state = MntToken::mnt_pool_state_storage(pool_id).borrow_state;
	let borrower_index = MntToken::mnt_borrower_index_storage(pool_id, borrower);
	assert_eq!(borrower_index, pool_state.mnt_distribution_index);

	assert_eq!(get_mnt_account_balance(borrower), expected_mnt_balance);
	assert_eq!(MntToken::mnt_accrued_storage(borrower), expected_mnt_in_storage);
}

/// Move flywheel and check supplier balance
fn check_supplier_accrued(
	pool_id: OriginalAsset,
	supplier: AccountId,
	expected_mnt_balance: Balance,
	expected_mnt_in_storage: Balance,
) {
	assert_ok!(MntToken::update_pool_mnt_supply_index(pool_id));
	assert_ok!(MntToken::distribute_supplier_mnt(pool_id, &supplier, false));
	assert_eq!(get_mnt_account_balance(supplier), expected_mnt_balance);
	assert_eq!(MntToken::mnt_accrued_storage(supplier), expected_mnt_in_storage);
}

#[test]
fn distribute_mnt_to_borrower_with_threshold() {
	ExtBuilder::default()
		.enable_minting_for_all_pools(10 * DOLLARS)
		.pool_borrow_underlying(DOT, 150_000 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(20)
		.pool_user_data(
			DOT,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5 too
			true,
		)
		.build()
		.execute_with(|| {
			// Award for ALICE is 10 per block
			// Threshold is 20
			// So at the first step awarded tokens should be kept in internal storage
			// At the second it should be transferred to ALICE and so on.

			let dot_speed = 10 * DOLLARS;
			assert_eq!(MntToken::mnt_speed_storage(DOT), dot_speed);
			assert_ok!(MntToken::update_pool_mnt_borrow_index(DOT));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));
			check_borrower(DOT, ALICE, 0, 0);

			System::set_block_number(2);
			// 2 tokens in internal storage
			check_borrower(DOT, ALICE, 0, dot_speed);

			System::set_block_number(3);
			// 4 tokens on account balance
			check_borrower(DOT, ALICE, dot_speed * 2, 0);

			System::set_block_number(4);
			// 2 tokens in internal storage and 4 tokens on account balance
			check_borrower(DOT, ALICE, dot_speed * 2, dot_speed);

			System::set_block_number(5);
			// 8 tokens on account balance
			check_borrower(DOT, ALICE, dot_speed * 4, 0);

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_mnt_to_supplier_with_threshold() {
	ExtBuilder::default()
		.enable_minting_for_all_pools(10 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(20)
		.pool_borrow_underlying(DOT, 100 * DOLLARS)
		.build()
		.execute_with(|| {
			// Award for ALICE is 10 per block
			// Threshold is 20
			// So at the first step awarded tokens should be kept in internal storage
			// At the second it should be transferred to ALICE and so on.

			let dot_speed = 10 * DOLLARS;
			assert_eq!(MntToken::mnt_speed_storage(DOT), dot_speed);

			// set total issuance
			Currencies::deposit(MDOT, &ALICE, 100 * DOLLARS).unwrap();

			check_supplier_accrued(DOT, ALICE, 0, dot_speed);
			System::set_block_number(2);
			check_supplier_accrued(DOT, ALICE, dot_speed * 2, 0);
			System::set_block_number(3);
			check_supplier_accrued(DOT, ALICE, dot_speed * 2, dot_speed);
			System::set_block_number(4);
			check_supplier_accrued(DOT, ALICE, dot_speed * 4, 0);
			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn distribute_mnt_to_supplier_from_different_pools() {
	ExtBuilder::default()
		.mnt_enabled_pools(vec![(DOT, 2 * DOLLARS), (KSM, 8 * DOLLARS)])
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_borrow_underlying(DOT, 100 * DOLLARS)
		.pool_borrow_underlying(KSM, 100 * DOLLARS)
		.build()
		.execute_with(|| {
			// Check accruing mnt tokens from two pools for supplier
			let dot_mnt_speed = 2 * DOLLARS;
			let ksm_mnt_speed = 8 * DOLLARS;
			assert_eq!(MntToken::mnt_speed_storage(DOT), dot_mnt_speed);
			assert_eq!(MntToken::mnt_speed_storage(KSM), ksm_mnt_speed);

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
		.enable_minting_for_all_pools(5 * DOLLARS)
		.pool_borrow_underlying(DOT, 150_000 * DOLLARS)
		.pool_borrow_underlying(KSM, 150_000 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_user_data(
			DOT,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
		)
		.pool_user_data(
			KSM,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
		)
		.build()
		.execute_with(|| {
			// First interaction with protocol for distributors.
			// This is a starting point to earn MNT token
			assert_ok!(MntToken::update_pool_mnt_borrow_index(DOT));
			assert_ok!(MntToken::update_pool_mnt_borrow_index(KSM));
			assert_ok!(MntToken::distribute_borrower_mnt(KSM, &ALICE, false));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));

			System::set_block_number(2);

			// Move flywheel
			assert_ok!(MntToken::update_pool_mnt_borrow_index(DOT));
			assert_ok!(MntToken::update_pool_mnt_borrow_index(KSM));
			assert_ok!(MntToken::distribute_borrower_mnt(KSM, &ALICE, false));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));

			// Total distributed to Alice: 5 from DOT + 5 from KSM
			assert_eq!(get_mnt_account_balance(ALICE), 10 * DOLLARS);

			let dot_mnt_speed = 5 * DOLLARS;
			// Check event about distributing mnt tokens by DOT pool
			let borrower_index = MntToken::mnt_borrower_index_storage(DOT, ALICE);
			let event = Event::MntToken(crate::Event::MntDistributedToBorrower(
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
		.enable_minting_for_all_pools(10 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_borrow_underlying(DOT, 150_000 * DOLLARS)
		.pool_user_data(
			DOT,
			ALICE,
			30_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
		)
		.pool_user_data(
			DOT,
			BOB,
			120_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5
			true,
		)
		.build()
		.execute_with(|| {
			/*
			Pool speed equals to 10
			Pool total borrow is 150_000. Alice borrowed 30_000 and BOB - 120_000

			This is a part of liquidity which belongs to Alice.
			30 / 150 = 0.2.

			10(mnt per block) * 0.2(alice part) = 2.
			This is how many MNT tokens per block Alice should acquire as a borrower.

			For Bob: 120 / 150 = 0.8; 0.8 * 10 = 8

			First interaction with protocol for distributors.
			This is started point to earn MNT token
			 */
			assert_ok!(MntToken::update_pool_mnt_borrow_index(DOT));
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
		.enable_minting_for_all_pools(12 * DOLLARS)
		.pool_borrow_underlying(DOT, 150_000 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		.pool_user_data(
			DOT,
			ALICE,
			150_000 * DOLLARS,
			Rate::saturating_from_rational(15, 10), // because pool borrow index is hardcoded to 1.5 too
			true,
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				MNT_PALLET_START_BALANCE,
				get_mnt_account_balance(MntToken::get_account_id())
			);
			let dot_speed = 12 * DOLLARS;
			// First interaction with protocol for distributors.
			// This is a starting point to earn MNT token
			assert_ok!(MntToken::update_pool_mnt_borrow_index(DOT));
			assert_ok!(MntToken::distribute_borrower_mnt(DOT, &ALICE, false));

			System::set_block_number(2);
			// Alice account borrow balance is 150_000
			check_borrower(DOT, ALICE, dot_speed, 0);

			// block_delta == 2
			System::set_block_number(4);
			check_borrower(DOT, ALICE, dot_speed * 3, 0);
			// check twice, move flywheel again
			check_borrower(DOT, ALICE, dot_speed * 3, 0);

			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn test_update_pool_mnt_borrow_index() {
	// TODO: check later
	ExtBuilder::default()
		.enable_minting_for_all_pools(10 * DOLLARS)
		.pool_borrow_underlying(DOT, 15_000 * DOLLARS)
		.pool_borrow_underlying(ETH, 30_000 * DOLLARS)
		.pool_borrow_underlying(KSM, 45_000 * DOLLARS)
		.pool_borrow_underlying(BTC, 60_000 * DOLLARS)
		.build()
		.execute_with(|| {
			let initial_index = Rate::one();
			System::set_block_number(1);

			let check_borrow_index = |pool_id: OriginalAsset, pool_mnt_speed: Balance, total_borrow: Balance| {
				MntToken::update_pool_mnt_borrow_index(pool_id).unwrap();
				// 1.5 current borrow_index. I use 15 in this function, that`s why I make total_borrow * 10
				let borrow_total_amount = Rate::saturating_from_rational(total_borrow * 10, 15);

				let expected_index = initial_index + Rate::from_inner(pool_mnt_speed) / borrow_total_amount;
				let pool_state = MntToken::mnt_pool_state_storage(pool_id);
				assert_eq!(pool_state.borrow_state.mnt_distribution_index, expected_index);
			};

			check_borrow_index(DOT, 10 * DOLLARS, 15_000);
			check_borrow_index(ETH, 10 * DOLLARS, 30_000);
			check_borrow_index(KSM, 10 * DOLLARS, 45_000);
			check_borrow_index(BTC, 10 * DOLLARS, 60_000);
		});
}

#[test]
fn test_update_pool_mnt_borrow_index_simple() {
	ExtBuilder::default()
		.enable_minting_for_all_pools(1 * DOLLARS)
		// total borrows needs to calculate mnt_speeds
		.pool_borrow_underlying(DOT, 150_000 * DOLLARS)
		.build()
		.execute_with(|| {
			/*
			* Minting was enabled when block_number was equal to 0. Here block_number == 1.
			So block_delta = 1

			Input parameters: 	dot_speed = 1,
								pool_borrowed = 150,
								pool_borrow_index = 1.5,
								mnt_acquired = delta_blocks * dot_speed = 1

			This is how much currency was borrowed without interest
			borrow_total_amount = pool_borrowed(150000) / pool_borrow_index(1.5)  = 100000

			How much MNT tokens were earned per block
			ratio = mnt_acquired / borrow_total_amount = 0.00001

			mnt_borrow_index = mnt_borrow_index(1 as initial value) + ratio(0.00001) = 1.00001

			*ratio is amount of MNT tokens for 1 borrowed token
			*/

			MntToken::update_pool_mnt_borrow_index(DOT).unwrap();
			let pool_state = MntToken::mnt_pool_state_storage(DOT);
			assert_eq!(
				pool_state.borrow_state.mnt_distribution_index,
				Rate::saturating_from_rational(100001, 100000)
			);
		});
}

#[test]
fn test_distribute_mnt_tokens_to_suppliers() {
	ExtBuilder::default()
		.enable_minting_for_all_pools(10 * DOLLARS)
		.mnt_account_balance(MNT_PALLET_START_BALANCE)
		.set_mnt_claim_threshold(0)
		// total borrows needs to calculate mnt_speeds
		.pool_borrow_underlying(DOT, 50 * DOLLARS)
		.build()
		.execute_with(|| {
			/*
			Minting was enabled when block_number was equal to 0. Here block_number == 1.
			So block_delta = 1

			Input parameters: 10 mnt speed per block for every pool.
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
				MntToken::update_pool_mnt_supply_index(DOT).unwrap();
				MntToken::distribute_supplier_mnt(DOT, &ALICE, false).unwrap();
				MntToken::distribute_supplier_mnt(DOT, &BOB, false).unwrap();
			};

			let check_supplier_award =
				|supplier_id: AccountId, distributed_amount: Balance, expected_user_mnt_balance: Balance| {
					let pool_state = MntToken::mnt_pool_state_storage(DOT);
					let supplier_index = MntToken::mnt_supplier_index_storage(DOT, supplier_id).unwrap();
					assert_eq!(supplier_index, pool_state.supply_state.mnt_distribution_index);
					assert_eq!(get_mnt_account_balance(supplier_id), expected_user_mnt_balance);
					// it should be 0 because threshold is 0
					assert_eq!(MntToken::mnt_accrued_storage(supplier_id), 0);

					let supplier_index = MntToken::mnt_supplier_index_storage(DOT, supplier_id).unwrap();
					let event = Event::MntToken(crate::Event::MntDistributedToSupplier(
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
				ALICE,
				alice_award_per_block * block_delta,
				alice_award_per_block * current_block,
			);
			assert_eq!(
				MNT_PALLET_START_BALANCE - get_mnt_account_balance(ALICE) - get_mnt_account_balance(BOB),
				get_mnt_account_balance(MntToken::get_account_id())
			)
		});
}

#[test]
fn test_update_pool_mnt_supply_index() {
	ExtBuilder::default()
		.enable_minting_for_all_pools(2 * DOLLARS)
		// total borrows needs to calculate mnt_speeds
		.pool_borrow_underlying(DOT, 50 * DOLLARS)
		.pool_borrow_underlying(ETH, 50 * DOLLARS)
		.pool_borrow_underlying(KSM, 50 * DOLLARS)
		.pool_borrow_underlying(BTC, 50 * DOLLARS)
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

			let check_supply_index = |pool_id: OriginalAsset, mnt_speed: Balance, total_issuance: Balance| {
				MntToken::update_pool_mnt_supply_index(pool_id).unwrap();
				let pool_state = MntToken::mnt_pool_state_storage(pool_id);
				assert_eq!(
					pool_state.supply_state.mnt_distribution_index,
					Rate::one() + Rate::from_inner(mnt_speed) / Rate::from_inner(total_issuance)
				);
				assert_eq!(pool_state.supply_state.index_updated_at_block, 1);
			};
			check_supply_index(DOT, 2 * DOLLARS, mdot_total_issuance);
			check_supply_index(KSM, 2 * DOLLARS, mksm_total_issuance);
			check_supply_index(ETH, 2 * DOLLARS, meth_total_issuance);
			check_supply_index(BTC, 2 * DOLLARS, mbtc_total_issuance);
		});
}

#[test]
fn test_update_pool_mnt_supply_index_simple() {
	ExtBuilder::default()
		// total_borrow shouldn't be zero at least for one market to calculate mnt speeds
		.pool_borrow_underlying(ETH, 150_000 * DOLLARS)
		.build()
		.execute_with(|| {
			// Input parameters:
			// supply_state.block_number = 1, supply_state.index = 1,
			// eth_speed = 10, total_supply = 20

			// set total_issuance to 20
			Currencies::deposit(METH, &ALICE, 20 * DOLLARS).unwrap();
			assert_ok!(MntToken::set_speed(admin_origin(), ETH, 10 * DOLLARS));

			System::set_block_number(2);
			MntToken::update_pool_mnt_supply_index(ETH).unwrap();
			let pool_state = MntToken::mnt_pool_state_storage(ETH);
			// block_delta = current_block(2) - supply_state.block_number(1) = 1
			// mnt_accrued = block_delta(1) * eth_speed(10) = 10
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
fn test_minting_enable_disable() {
	let check_mnt_storage = |pool_id, speed, borrow_index, supply_index, block_number| {
		assert_eq!(MntToken::mnt_speed_storage(pool_id), speed);
		assert_eq!(
			MntToken::mnt_pool_state_storage(pool_id),
			MntPoolState {
				supply_state: MntState {
					mnt_distribution_index: supply_index,
					index_updated_at_block: block_number
				},
				borrow_state: MntState {
					mnt_distribution_index: borrow_index,
					index_updated_at_block: block_number
				}
			}
		);
	};
	ExtBuilder::default()
		.user_balance(ADMIN, MDOT, 100 * DOLLARS)
		.pool_borrow_underlying(DOT, 50 * DOLLARS)
		.pool_borrow_underlying(KSM, 50 * DOLLARS)
		.mnt_account_balance(100 * DOLLARS)
		.build()
		.execute_with(|| {
			// The dispatch origin of this call must be Root or 2/3 MinterestCouncil.
			assert_noop!(MntToken::set_speed(alice_origin(), DOT, 1 * DOLLARS), BadOrigin);

			// Unable to enable minting for non existing pool
			assert_noop!(
				MntToken::set_speed(admin_origin(), ETH, 2 * DOLLARS),
				Error::<Runtime>::PoolNotFound
			);

			// Enable the distribution of MNT tokens in the DOT liquidity pool
			let dot_speed = 2 * DOLLARS;
			assert_ok!(MntToken::set_speed(admin_origin(), DOT, dot_speed));
			let speed_changed_event = Event::MntToken(crate::Event::MntSpeedChanged(DOT, dot_speed));
			assert!(System::events()
				.iter()
				.any(|record| record.event == speed_changed_event));
			check_mnt_storage(DOT, dot_speed, Rate::one(), Rate::one(), 1);

			System::set_block_number(5);

			// Unable to disable an already disabled pool
			assert_noop!(
				MntToken::set_speed(admin_origin(), KSM, Balance::zero()),
				Error::<Runtime>::MntMintingNotEnabled
			);

			// Enable the distribution of MNT tokens in the KSM liquidity pool
			let ksm_speed = 2 * DOLLARS;
			assert_ok!(MntToken::set_speed(admin_origin(), KSM, ksm_speed));
			let speed_changed_event = Event::MntToken(crate::Event::MntSpeedChanged(KSM, ksm_speed));
			assert!(System::events()
				.iter()
				.any(|record| record.event == speed_changed_event));
			check_mnt_storage(KSM, ksm_speed, Rate::one(), Rate::one(), 5);

			System::set_block_number(10);

			// Disable the distribution of MNT tokens in the DOT liquidity pool
			assert_ok!(MntToken::set_speed(admin_origin(), DOT, Balance::zero()));
			let speed_changed_event = Event::MntToken(crate::Event::MntSpeedChanged(DOT, Balance::zero()));
			assert!(System::events()
				.iter()
				.any(|record| record.event == speed_changed_event));
			assert!(!crate::MntSpeedStorage::<Runtime>::contains_key(DOT));
			check_mnt_storage(
				DOT,
				Balance::zero(),
				Rate::from_inner(1_540000000000000000),
				Rate::from_inner(1_180000000000000000),
				10,
			);

			System::set_block_number(15);

			assert_ok!(MntToken::update_pool_mnt_supply_index(DOT));
			assert_ok!(MntToken::update_pool_mnt_borrow_index(DOT));
			// Check that indices hadn't been updated while distribution is off
			check_mnt_storage(
				DOT,
				Balance::zero(),
				Rate::from_inner(1_540000000000000000),
				Rate::from_inner(1_180000000000000000),
				10,
			);

			System::set_block_number(20);

			// Enable the distribution of MNT tokens in the DOT liquidity pool
			// Check that the indexes have been saved and the block number has changed.
			assert_ok!(MntToken::set_speed(admin_origin(), DOT, dot_speed));
			check_mnt_storage(
				DOT,
				dot_speed,
				Rate::from_inner(1_540000000000000000),
				Rate::from_inner(1_180000000000000000),
				20,
			);

			// Change the mnt_speed parameter for KSM liquidity pool.
			// Check  that the indexes have been updated and block number has changed.
			assert_ok!(MntToken::set_speed(admin_origin(), KSM, ksm_speed + 1_u128));
			check_mnt_storage(
				KSM,
				ksm_speed + 1_u128,
				Rate::from_inner(1_900000000000000000),
				Rate::from_inner(1_000000000000000000),
				20,
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
			assert_eq!(MntToken::mnt_accrued_storage(ALICE), first_transfer);

			// distribute_all == true, user_accrued > threshold:
			// we perform the transfer.
			let second_transfer = 200 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, second_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer);
			assert_eq!(MntToken::mnt_accrued_storage(ALICE), Balance::zero());

			// distribute_all == true, user_accrued == 0:
			// we do not perform the transfer.
			let third_transfer = Balance::zero();
			assert_ok!(MntToken::transfer_mnt(&ALICE, third_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer);
			assert_eq!(MntToken::mnt_accrued_storage(ALICE), Balance::zero());

			// distribute_all == true, user_accrued > threshold, user_accrued > MNT_pallet_balance:
			// we do not perform the transfer.
			let fourth_transfer = 10_000_000 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, fourth_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer);
			assert_eq!(MntToken::mnt_accrued_storage(ALICE), Balance::zero());

			// distribute_all == true, user_accrued < threshold:
			// we perform the transfer.
			let fifth_transfer = 10 * DOLLARS;
			assert_ok!(MntToken::transfer_mnt(&ALICE, first_transfer, true));
			assert_eq!(
				get_mnt_account_balance(MntToken::get_account_id()),
				MNT_PALLET_START_BALANCE - second_transfer - fifth_transfer
			);
			assert_eq!(get_mnt_account_balance(ALICE), second_transfer + fifth_transfer);
			assert_eq!(MntToken::mnt_accrued_storage(ALICE), Balance::zero());

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
			assert_eq!(MntToken::mnt_accrued_storage(ALICE), Balance::zero());
		});
}
