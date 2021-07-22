//! Unit tests for the vesting module.

use super::*;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use minterest_primitives::constants::currency::DOLLARS;
use minterest_primitives::{Balance, BlockNumber};
use mock::{Event, *};
use pallet_balances::{BalanceLock, Reasons};

#[test]
fn vesting_from_chain_spec_works() {
	// Charlie has a schedule from the Private Sale bucket set in the genesis block.
	let private_sale_schedule = VestingSchedule {
		bucket: VestingBucket::PrivateSale,
		start: 0u64,
		period: 1u64,
		period_count: BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(3805175038051_750380517503805175), // total = 20 MNT
	};
	ExtBuilder::build().execute_with(|| {
		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			10 * DOLLARS,
			WithdrawReasons::TRANSFER,
			20 * DOLLARS
		));
		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 10 * DOLLARS);

		assert_eq!(
			Vesting::vesting_schedule_storage(&CHARLIE::get()),
			vec![private_sale_schedule]
		);

		// Set the block number equal to half a year and do claim().
		System::set_block_number(BLOCKS_PER_YEAR as u64 / 2);
		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));

		// // Should be usable 10 MNT from Private Sale bucket.
		// (-1 written due to math problems)
		let expected_event = Event::Vesting(crate::Event::Claimed(CHARLIE::get(), 10 * DOLLARS - 1));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			20 * DOLLARS,
			WithdrawReasons::TRANSFER,
			10 * DOLLARS
		));
		// 10 MNT free + 10 MNT from Private Sale bucket. (+1 written due to math problems)
		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 20 * DOLLARS + 1);

		// Set the block number equal to a year and do claim().
		System::set_block_number(BLOCKS_PER_YEAR as u64);
		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));

		// Should be usable 20 MNT from Private Sale bucket and 10 MNT free from genesis.
		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 30 * DOLLARS);
		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			30 * DOLLARS,
			WithdrawReasons::TRANSFER,
			Balance::zero()
		));
		// All schedules are finished. All tokens are usable.
		assert_eq!(PalletBalances::locks(&CHARLIE::get()), vec![]);
	});
}

#[test]
fn vested_transfer_works() {
	let team_schedule = VestingSchedule {
		bucket: VestingBucket::Team,
		start: 0u64,
		period: 1u64,
		period_count: 5 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(388127853881_278538812785388127), // total = 10.2 MNT
	};

	ExtBuilder::build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			10_200_000_000_000_000_000 // 10.2 MNT
		));

		// There are 1 active vesting schedule.
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![team_schedule.clone()]
		);

		let vested_event = Event::Vesting(crate::Event::VestingScheduleAdded(BOB::get(), team_schedule));
		assert!(System::events().iter().any(|record| record.event == vested_event));

		// Team vesting bucket balance equal 989.8 MNT.
		// (+1 written due to math problems)
		assert_eq!(
			PalletBalances::free_balance(BucketTeam::get()),
			989_800_000_000_000_000_000 + 1
		);
		// BOB should receive 10.2 tokens at the end of schedule.
		// (-1 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 10_200_000_000_000_000_000 - 1);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());
	});
}

#[test]
fn add_new_vesting_schedule_merges_with_current_locked_balance_and_until() {
	let team_schedule = VestingSchedule {
		bucket: VestingBucket::Team,
		start: 0u64,
		period: 1u64,
		period_count: 5 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(761035007610_350076103500761035), // total = 20 MNT
	};
	let team_schedule_2 = VestingSchedule {
		bucket: VestingBucket::Team,
		start: 13_140_000_u64,
		period: 1u64,
		period_count: 5 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(266362252663_622526636225266362), // total = 7 MNT
	};

	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			20 * DOLLARS
		));
		// There are 1 active vesting schedules for BOB.
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![team_schedule.clone()]
		);

		// Set the block number equal to 2.5 years.
		// Half of the vesting period for the bucket Team.
		System::set_block_number(BLOCKS_PER_YEAR as u64 * 5 / 2);

		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			13_140_000_u64,
			7 * DOLLARS
		));
		// There are 2 active vesting schedules for BOB.
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![team_schedule.clone(), team_schedule_2.clone()]
		);

		// PalletBalances::locks(&BOB::get())
		// Should be locked 10 MNT from first team schedule and 7 MNT from second team schedule.
		// (-2 written due to math problems)
		assert_eq!(
			PalletBalances::locks(&BOB::get())[0],
			BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 17 * DOLLARS - 2,
				reasons: Reasons::All,
			}
		);
	});
}

#[test]
fn cannot_use_fund_if_not_claimed() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Marketing,
			10u64,
			50 * DOLLARS
		));
		assert!(PalletBalances::ensure_can_withdraw(
			&BOB::get(),
			10 * DOLLARS,
			WithdrawReasons::TRANSFER,
			40 * DOLLARS
		)
		.is_err());
	});
}

#[test]
fn vested_transfer_fails_if_transfer_err() {
	ExtBuilder::build().execute_with(|| {
		// The Team vesting bucket on the account has a 1000 MNT.
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				1u64,
				1001 * DOLLARS
			),
			pallet_balances::Error::<TestRuntime, _>::InsufficientBalance,
		);
	});
}

#[test]
fn vested_transfer_fails_if_overflow() {
	ExtBuilder::build().execute_with(|| {
		// start = u64::MAX
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				u64::MAX,
				2 * DOLLARS
			),
			Error::<TestRuntime>::NumOverflow
		);
	});
}

#[test]
fn vested_transfer_and_remove_fails_if_bad_origin() {
	ExtBuilder::build().execute_with(|| {
		assert_noop!(
			Vesting::remove_vesting_schedules(Origin::signed(ALICE::get()), CHARLIE::get(), VestingBucket::PrivateSale),
			BadOrigin
		);
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ALICE::get()),
				BOB::get(),
				VestingBucket::StrategicPartners,
				0u64,
				100 * DOLLARS
			),
			BadOrigin
		);
	});
}

#[test]
fn vested_transfer_and_remove_fails_if_incorrect_bucket_type() {
	// Charlie has a schedule from the Private Sale bucket set in the genesis block.
	let private_sale_schedule = VestingSchedule {
		bucket: VestingBucket::PrivateSale,
		start: 0u64,
		period: 1u64,
		period_count: BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(3805175038051_750380517503805175), // total = 20 MNT
	};

	ExtBuilder::build().execute_with(|| {
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::PublicSale,
				0u64,
				100 * DOLLARS
			),
			Error::<TestRuntime>::IncorrectVestingBucketType
		);
		assert_noop!(
			Vesting::remove_vesting_schedules(Origin::signed(ADMIN::get()), CHARLIE::get(), VestingBucket::PrivateSale),
			Error::<TestRuntime>::IncorrectVestingBucketType
		);
		// After a failed deletion, the schedule should remain on the account.
		assert_eq!(
			Vesting::vesting_schedule_storage(&CHARLIE::get()),
			vec![private_sale_schedule]
		);
		assert_eq!(
			PalletBalances::locks(&CHARLIE::get())[0],
			BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 20 * DOLLARS,
				reasons: Reasons::All,
			}
		);
	});
}

#[test]
fn claim_works() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Marketing,
			0u64,
			20 * DOLLARS
		));

		// 20.0 MNT. (-1 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 20 * DOLLARS - 1);

		// Set the block number equal to half a year. Should be usable 10 MNT from Marketing bucket.
		System::set_block_number(BLOCKS_PER_YEAR as u64 / 2);

		// Remain locked if not claimed.
		assert!(PalletBalances::transfer(Origin::signed(BOB::get()), ALICE::get(), 10 * DOLLARS).is_err());

		// Unlocked after claiming.
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_ok!(PalletBalances::transfer(
			Origin::signed(BOB::get()),
			ALICE::get(),
			10 * DOLLARS
		));

		// More are still locked.
		assert!(PalletBalances::transfer(Origin::signed(BOB::get()), ALICE::get(), 1 * DOLLARS).is_err());

		// Set the block number equal to a year. Schedule from Marketing bucket is over.
		System::set_block_number(BLOCKS_PER_YEAR as u64);

		// Claim more.
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_ok!(PalletBalances::transfer(
			Origin::signed(BOB::get()),
			ALICE::get(),
			10 * DOLLARS - 1 // 10.0 MNT. (-1 written due to math problems)
		));
		// All used up.
		assert_eq!(PalletBalances::free_balance(BOB::get()), Balance::zero());

		// Schedule is finished. No locks anymore.
		assert_eq!(PalletBalances::locks(&BOB::get()), vec![]);
	});
}

#[test]
fn vested_transfer_check_for_min() {
	ExtBuilder::build().execute_with(|| {
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				1u64,
				3 * DOLLARS
			),
			Error::<TestRuntime>::AmountLow
		);
	});
}

#[test]
fn multiple_vesting_schedule_claim_works() {
	let marketing_schedule = VestingSchedule {
		bucket: VestingBucket::Marketing,
		start: 0u64,
		period: 1u64,
		period_count: BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(3805175038051_750380517503805175), // total = 20 MNT
	};
	let strategic_partners_schedule = VestingSchedule {
		bucket: VestingBucket::StrategicPartners,
		start: 0u64,
		period: 1u64,
		period_count: 2 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(2853881278538_812785388127853881), // total = 30 MNT
	};

	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Marketing,
			0u64,
			20 * DOLLARS
		));
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::StrategicPartners,
			0u64,
			30 * DOLLARS
		));

		// There are 2 active vesting schedules for BOB.
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![marketing_schedule.clone(), strategic_partners_schedule.clone()]
		);

		// BOB should receive 50 tokens at the end of all schedules.
		// (-2 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());

		// Set the block number equal to half a year and do claim().
		System::set_block_number(BLOCKS_PER_YEAR as u64 / 2);
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));

		// Should be usable 10 MNT from Marketing bucket and 7.5 MNT from Strategic Partners bucket.
		// (-2 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 17_500_000_000_000_000_000);

		// There are 2 active vesting schedules for BOB.
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![marketing_schedule, strategic_partners_schedule.clone()]
		);

		// Set the block number equal to a year.
		System::set_block_number(BLOCKS_PER_YEAR as u64);

		// Schedule from Marketing bucket is over.
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![strategic_partners_schedule]
		);

		// Should be usable 20 MNT from Marketing bucket and 15 MNT from Strategic Partners bucket.
		// (-2 and -1 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 35 * DOLLARS - 1);

		// Set the block number equal to two years.
		System::set_block_number(2 * BLOCKS_PER_YEAR as u64);

		// All schedules are finished. All tokens are usable.
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		// (-2 and written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(VestingScheduleStorage::<TestRuntime>::contains_key(&BOB::get()), false);
		assert_eq!(PalletBalances::locks(&BOB::get()), vec![]);
	});
}

#[test]
fn remove_vesting_schedule_should_work() {
	let private_sale_schedule = VestingSchedule {
		bucket: VestingBucket::PrivateSale,
		start: 0u64,
		period: 1u64,
		period_count: BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(3805175038051_750380517503805175), // total = 20 MNT
	};
	let strategic_partners_schedule = VestingSchedule {
		bucket: VestingBucket::StrategicPartners,
		start: 0u64,
		period: 1u64,
		period_count: 2 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(89421613394216_133942161339421613), // total = 940 MNT
	};
	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			CHARLIE::get(),
			VestingBucket::StrategicPartners,
			0u64,
			940 * DOLLARS
		));

		// There are 2 active vesting schedules for CHARLIE.
		assert_eq!(
			Vesting::vesting_schedule_storage(&CHARLIE::get()),
			vec![private_sale_schedule.clone(), strategic_partners_schedule.clone()]
		);

		// Strategic Partners vesting bucket balance equal: 1000 MNT - 940 MNT = 60.0 MNT
		assert_eq!(
			PalletBalances::free_balance(BucketStrategicPartners::get()),
			60_000_000_000_000_000_000
		);

		// Set the block number equal to half a year and do claim().
		System::set_block_number(BLOCKS_PER_YEAR as u64 / 2);
		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));

		// CHARLIE free balance should be equal 10.0 MNT from genesis +
		// + 940.0 MNT from Strategic Partners bucket + 20.0 MNT from Private Sale bucket
		assert_eq!(
			PalletBalances::free_balance(CHARLIE::get()),
			970_000_000_000_000_000_000
		);
		// Should be usable 10.0 MNT from Private Sale bucket, 235.0 MNT from Strategic Partners bucket.
		// and 10.0 MNT from genesis block (-1 written due to math problems)
		assert_eq!(
			PalletBalances::usable_balance(CHARLIE::get()),
			255_000_000_000_000_000_000 + 1
		);

		// There are 2 active vesting schedules for CHARLIE.
		assert_eq!(
			Vesting::vesting_schedule_storage(&CHARLIE::get()),
			vec![private_sale_schedule.clone(), strategic_partners_schedule]
		);

		// Set the block number equal to 9 months and remove schedules.
		System::set_block_number(BLOCKS_PER_YEAR as u64 / 12 * 9);

		assert_ok!(Vesting::remove_vesting_schedules(
			Origin::signed(ADMIN::get()),
			CHARLIE::get(),
			VestingBucket::StrategicPartners,
		));

		// CHARLIE free balance should be equal 10.0 MNT from genesis +
		// + (940.0 - 587.5) MNT from Strategic Partners bucket + 20.0 MNT from Private Sale bucket.
		assert_eq!(
			PalletBalances::free_balance(CHARLIE::get()),
			382_500_000_000_000_000_000
		);
		// CHARLIE usable balance should be equal 352.5 MNT from Strategic Partners bucket,
		// 15.0 MNT from Private Sale bucket and 10.0 MNT from genesis block.
		assert_eq!(
			PalletBalances::usable_balance(CHARLIE::get()),
			377_500_000_000_000_000_000 + 1
		);

		// Strategic partners schedule is removed.
		assert_eq!(
			Vesting::vesting_schedule_storage(&CHARLIE::get()),
			vec![private_sale_schedule]
		);

		// CHARLIE doesn't have schedule from Strategic Partners vesting bucket.
		assert_noop!(
			Vesting::remove_vesting_schedules(
				Origin::signed(ADMIN::get()),
				CHARLIE::get(),
				VestingBucket::StrategicPartners,
			),
			Error::<TestRuntime>::UserDoesNotHaveSuchSchedule
		);

		// Strategic Partners vesting bucket balance equal: 60.0 MNT + 587.5 MNT = 647.5 MNT
		assert_eq!(
			PalletBalances::free_balance(BucketStrategicPartners::get()),
			647_500_000_000_000_000_000
		);
	});
}

#[test]
fn remove_vesting_schedules_from_one_bucket_should_work() {
	let team_schedule1 = VestingSchedule {
		bucket: VestingBucket::Team,
		start: 0u64,
		period: 1u64,
		period_count: 5 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(19025875190258_751902587519025875), // total = 500 MNT
	};
	let team_schedule2 = VestingSchedule {
		bucket: VestingBucket::Team,
		start: 1000u64,
		period: 1u64,
		period_count: 5 * BLOCKS_PER_YEAR as u32,
		per_period: Rate::from_inner(3805175038051_750380517503805175), // total = 100 MNT
	};

	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			500 * DOLLARS
		));
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			1000u64,
			100 * DOLLARS
		));

		// There are 2 active vesting schedules for BOB.
		assert_eq!(
			Vesting::vesting_schedule_storage(&BOB::get()),
			vec![team_schedule1.clone(), team_schedule2.clone()]
		);

		assert_ok!(Vesting::remove_vesting_schedules(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
		));

		// All schedules are removed.
		assert_eq!(Vesting::vesting_schedule_storage(&BOB::get()), vec![]);
	});
}

#[test]
fn vesting_schedule_constructors_should_work() {
	let schedule1: VestingSchedule<BlockNumber> = VestingSchedule::new(VestingBucket::Ecosystem, DOLLARS);
	assert_eq!(schedule1.bucket, VestingBucket::Ecosystem);
	assert_eq!(schedule1.start, BlockNumber::zero());
	assert_eq!(schedule1.period_count as u128, 4 * BLOCKS_PER_YEAR);
	assert_eq!(schedule1.period, 1_u32);
	// 1 MNT / 21_024_000 blocks ~ 0,0000000476
	assert_eq!(schedule1.per_period, Rate::from_inner(47564687975_646879756468797564));

	let schedule2: VestingSchedule<BlockNumber> = VestingSchedule::new(VestingBucket::Team, 100_000 * DOLLARS);
	assert_eq!(schedule2.bucket, VestingBucket::Team);
	assert_eq!(schedule2.start, 2_620_800);
	assert_eq!(schedule2.period_count as u128, 5 * BLOCKS_PER_YEAR);
	assert_eq!(schedule2.period, 1_u32);
	// 100_000 MNT / 26_280_000 blocks ~ 0,0038
	assert_eq!(
		schedule2.per_period,
		Rate::from_inner(3805175038051750_380517503805175038)
	);

	let schedule3: VestingSchedule<BlockNumber> =
		VestingSchedule::new_beginning_from(VestingBucket::Marketing, 1234, 10 * DOLLARS).unwrap();
	assert_eq!(schedule3.bucket, VestingBucket::Marketing);
	assert_eq!(schedule3.start, 1234);
	assert_eq!(schedule3.period_count as u128, BLOCKS_PER_YEAR);
	assert_eq!(schedule3.period, 1_u32);
	// 10 MNT / 5256000 blocks ~ 0,0000019
	assert_eq!(schedule3.per_period, Rate::from_inner(1902587519025_875190258751902587));

	let schedule4: VestingSchedule<BlockNumber> =
		VestingSchedule::new_beginning_from(VestingBucket::StrategicPartners, 5000, 20 * DOLLARS).unwrap();
	assert_eq!(schedule4.bucket, VestingBucket::StrategicPartners);
	assert_eq!(schedule4.start, 5000);
	assert_eq!(schedule4.period_count as u128, 2 * BLOCKS_PER_YEAR);
	assert_eq!(schedule4.period, 1_u32);
	// 20 MNT / 10512000 blocks ~ 0,0000019
	assert_eq!(schedule4.per_period, Rate::from_inner(1902587519025_875190258751902587));
}
