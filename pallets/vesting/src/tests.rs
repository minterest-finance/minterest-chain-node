//! Unit tests for the vesting module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use minterest_primitives::constants::currency::DOLLARS;
use minterest_primitives::{Balance, BlockNumber};
use mock::{Event, *};
use pallet_balances::{BalanceLock, Reasons};

#[test]
fn vesting_from_chain_spec_works() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			10 * DOLLARS,
			WithdrawReasons::TRANSFER,
			20 * DOLLARS
		));
		assert!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			11 * DOLLARS,
			WithdrawReasons::TRANSFER,
			19 * DOLLARS
		)
		.is_err());
		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 10 * DOLLARS);

		assert_eq!(
			Vesting::vesting_schedules(&CHARLIE::get()),
			vec![VestingSchedule {
				bucket: VestingBucket::Team,
				start: 2620800u64,
				period: 1u64,
				period_count: 26280000u32,
				per_period: Rate::from_inner(761035007610_350076103500761035),
			}]
		);

		System::set_block_number(13);

		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));
		// 10 MNT. (-1 written due to math problems)
		let expected_event = Event::vesting(crate::Event::Claimed(CHARLIE::get(), 10 * DOLLARS - 1));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			20 * DOLLARS,
			WithdrawReasons::TRANSFER,
			10 * DOLLARS
		));
		assert!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			21 * DOLLARS,
			WithdrawReasons::TRANSFER,
			9 * DOLLARS
		)
		.is_err());
		// 20 MNT. (+1 written due to math problems)
		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 20 * DOLLARS + 1);

		System::set_block_number(14);

		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));

		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 30 * DOLLARS);
		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			30 * DOLLARS,
			WithdrawReasons::TRANSFER,
			Balance::zero()
		));
	});
}

#[test]
fn vested_transfer_works() {
	ExtBuilder::build().execute_with(|| {
		System::set_block_number(1);

		let schedule = VestingSchedule {
			start: 0u64,
			period: 1u64,
			period_count: 26280000u32,
			per_period: Rate::from_inner(388127853881_278538812785388127),
		};

		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			10_200_000_000_000_000_000 // 10.2 MNT
		));
		assert_eq!(Vesting::vesting_schedules(&BOB::get()), vec![schedule.clone()]);

		let vested_event = Event::vesting(crate::Event::VestingScheduleAdded(BOB::get(), schedule));
		assert!(System::events().iter().any(|record| record.event == vested_event));

		// 989.8 MNT. (+1 written due to math problems)
		assert_eq!(
			PalletBalances::free_balance(BucketTeam::get()),
			989_800_000_000_000_000_000 + 1
		);
		// 10.2 MNT. (-1 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 10_200_000_000_000_000_000 - 1);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());
	});
}

#[test]
fn add_new_vesting_schedule_merges_with_current_locked_balance_and_until() {
	ExtBuilder::build().execute_with(|| {
		let first_schedule = VestingSchedule {
			bucket: VestingBucket::Team,
			start: 0u64,
			period: 1u64,
			period_count: 26280000u32,
			per_period: Rate::from_inner(761035007610_350076103500761035),
		};
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			20 * DOLLARS
		));
		// Check vesting schedules for BOB::get()
		assert_eq!(Vesting::vesting_schedules(&BOB::get()), vec![first_schedule.clone()]);

		// Half of the vesting period for the bucket Team
		System::set_block_number(13_140_000);

		let second_schedule = VestingSchedule {
			bucket: VestingBucket::Team,
			start: 13_140_000_u64,
			period: 1u64,
			period_count: 26280000u32,
			per_period: Rate::from_inner(266362252663_622526636225266362),
		};
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			13_140_000_u64,
			7 * DOLLARS
		));
		// Check vesting schedules for BOB::get()
		assert_eq!(
			Vesting::vesting_schedules(&BOB::get()),
			vec![first_schedule.clone(), second_schedule.clone()]
		);
		// bob_locks = 20 / 2 + 7 = 17.0 MNT
		assert_eq!(
			PalletBalances::locks(&BOB::get()).pop(),
			Some(BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 17 * DOLLARS - 2, // 17.0 MNT. (-2 written due to math problems)
				reasons: Reasons::All,
			})
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
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				1u64,
				1001 * DOLLARS
			),
			pallet_balances::Error::<Runtime, _>::InsufficientBalance,
		);
	});
}

#[test]
fn vested_transfer_fails_if_overflow() {
	ExtBuilder::build().execute_with(|| {
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				u64::MAX,
				2 * DOLLARS
			),
			Error::<Runtime>::NumOverflow
		);
	});
}

#[test]
fn vested_transfer_fails_if_bad_origin() {
	ExtBuilder::build().execute_with(|| {
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
fn vested_transfer_fails_if_incorrect_bucket_type() {
	ExtBuilder::build().execute_with(|| {
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::PublicSale,
				0u64,
				100 * DOLLARS
			),
			Error::<Runtime>::IncorrectVestingBucketType
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

		// Half of the vesting period for the bucket Marketing
		System::set_block_number(2_628_000);
		// remain locked if not claimed
		assert!(PalletBalances::transfer(Origin::signed(BOB::get()), ALICE::get(), 10 * DOLLARS).is_err());
		// unlocked after claiming
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_ok!(PalletBalances::transfer(
			Origin::signed(BOB::get()),
			ALICE::get(),
			10 * DOLLARS
		));
		// more are still locked
		assert!(PalletBalances::transfer(Origin::signed(BOB::get()), ALICE::get(), 1 * DOLLARS).is_err());

		System::set_block_number(5256000);
		// claim more
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_ok!(PalletBalances::transfer(
			Origin::signed(BOB::get()),
			ALICE::get(),
			10 * DOLLARS - 1 // 10.0 MNT. (-1 written due to math problems)
		));
		// all used up
		assert_eq!(PalletBalances::free_balance(BOB::get()), Balance::zero());

		// no locks anymore
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
			Error::<Runtime>::AmountLow
		);
	});
}

#[test]
fn multiple_vesting_schedule_claim_works() {
	let marketing_schedule = VestingSchedule {
		bucket: VestingBucket::Marketing,
		start: 0u64,
		period: 1u64,
		period_count: 5256000u32,                                       // 1 year
		per_period: Rate::from_inner(3805175038051_750380517503805175), // total = 20 MNT
	};
	let strategic_partners_schedule = VestingSchedule {
		bucket: VestingBucket::StrategicPartners,
		start: 0u64,
		period: 1u64,
		period_count: 10512000u32,                                      // 2 years
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

		// There are 2 active vesting schedules for BOB::get()
		assert_eq!(
			Vesting::vesting_schedules(&BOB::get()),
			vec![marketing_schedule.clone(), strategic_partners_schedule.clone()]
		);

		// BOB should receive 50 tokens at the end of all schedules
		// (-2 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());

		// Set the block number equal to half a year and do claim().
		System::set_block_number(2628000);
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));

		// Should be usable 10 MNT from Marketing bucket and 7.5 MNT from Strategic Partners bucket.
		// (-2 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 17_500_000_000_000_000_000);

		// There are 2 active vesting schedules
		assert_eq!(
			Vesting::vesting_schedules(&BOB::get()),
			vec![marketing_schedule, strategic_partners_schedule.clone()]
		);

		// Set the block number equal to a year.
		System::set_block_number(5_256_000);

		// Schedule from Marketing bucket is over.
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(
			Vesting::vesting_schedules(&BOB::get()),
			vec![strategic_partners_schedule]
		);

		// Should be usable 20 MNT from Marketing bucket and 15 MNT from Strategic Partners bucket.
		// (-2 and -1 written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 35 * DOLLARS - 1);

		// Set the block number equal to two years.
		System::set_block_number(10_512_000);

		// All schedules are finished. All tokens are usable
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		// (-2 and written due to math problems)
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 50 * DOLLARS - 2);
		assert_eq!(VestingSchedules::<Runtime>::contains_key(&BOB::get()), false);
		assert_eq!(PalletBalances::locks(&BOB::get()), vec![]);
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
		VestingSchedule::new_beginning_from(VestingBucket::Marketing, 1234, 10 * DOLLARS);
	assert_eq!(schedule3.bucket, VestingBucket::Marketing);
	assert_eq!(schedule3.start, 1234);
	assert_eq!(schedule3.period_count as u128, BLOCKS_PER_YEAR);
	assert_eq!(schedule3.period, 1_u32);
	// 1 MNT / 5256000 blocks ~ 0,00000019
	assert_eq!(schedule3.per_period, Rate::from_inner(1902587519025_875190258751902587));

	let schedule4: VestingSchedule<BlockNumber> =
		VestingSchedule::new_beginning_from(VestingBucket::StrategicPartners, 5000, 20 * DOLLARS);
	assert_eq!(schedule4.bucket, VestingBucket::StrategicPartners);
	assert_eq!(schedule4.start, 5000);
	assert_eq!(schedule4.period_count as u128, 2 * BLOCKS_PER_YEAR);
	assert_eq!(schedule4.period, 1_u32);
	// 20 MNT / 10512000 blocks ~ 0,0000019
	assert_eq!(schedule4.per_period, Rate::from_inner(1902587519025_875190258751902587));
}
