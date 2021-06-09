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
				per_period: 761035007610,
			}]
		);

		// Half vesting duration for Team's vesting bucket
		System::set_block_number(15760800);

		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));
		// ~ 10 MNT
		let expected_event = Event::vesting(crate::Event::Claimed(CHARLIE::get(), 9_999_999_999_995_400_000));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			20_000_000_000_004_600_000,
			WithdrawReasons::TRANSFER,
			9_999_999_999_995_400_000
		));
		assert!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			21 * DOLLARS,
			WithdrawReasons::TRANSFER,
			9 * DOLLARS
		)
		.is_err());
		assert_eq!(
			PalletBalances::usable_balance(CHARLIE::get()),
			20_000_000_000_004_600_000
		);

		// The entire period of vesting from the team bucket has passed
		System::set_block_number(28900800);

		assert_ok!(Vesting::claim(Origin::signed(CHARLIE::get())));

		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE::get(),
			30 * DOLLARS,
			WithdrawReasons::TRANSFER,
			Balance::zero()
		));
		assert_eq!(PalletBalances::usable_balance(CHARLIE::get()), 30 * DOLLARS);
	});
}

#[test]
fn vested_transfer_works() {
	ExtBuilder::build().execute_with(|| {
		System::set_block_number(1);

		let schedule = VestingSchedule {
			bucket: VestingBucket::Team,
			start: 0u64,
			period: 10u64,
			period_count: 1u32,
			per_period: 100 * DOLLARS,
		};
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			100 * DOLLARS
		));
		assert_eq!(Vesting::vesting_schedules(&BOB::get()), vec![schedule.clone()]);

		let vested_event = Event::vesting(crate::Event::VestingScheduleAdded(BOB::get(), schedule));
		assert!(System::events().iter().any(|record| record.event == vested_event));

		assert_eq!(PalletBalances::free_balance(ALICE::get()), Balance::zero());
		assert_eq!(PalletBalances::free_balance(BOB::get()), 100 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());

		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(PalletBalances::free_balance(BOB::get()), 100 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());

		System::set_block_number(10);

		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(PalletBalances::free_balance(BOB::get()), 100 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 100 * DOLLARS);
	});
}

#[test]
fn add_new_vesting_schedule_merges_with_current_locked_balance_and_until() {
	ExtBuilder::build().execute_with(|| {
		assert_eq!(
			PalletBalances::free_balance(VestingBucket::Marketing.bucket_account_id().unwrap()),
			1000 * DOLLARS
		);

		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			20 * DOLLARS
		));

		assert_eq!(PalletBalances::free_balance(BOB::get()), 20 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());

		System::set_block_number(12);

		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			10u64,
			7 * DOLLARS
		));

		assert_eq!(
			PalletBalances::locks(&BOB::get()).pop(),
			Some(BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 17 * DOLLARS,
				reasons: Reasons::All,
			})
		);

		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				10u64,
				7 * DOLLARS
			),
			Error::<Runtime>::TooManyVestingSchedules
		);

		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(PalletBalances::free_balance(BOB::get()), 27 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 10 * DOLLARS);

		System::set_block_number(23);

		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 27 * DOLLARS);
	});
}

#[test]
fn cannot_use_fund_if_not_claimed() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			10u64,
			50 * DOLLARS
		));
		assert!(PalletBalances::ensure_can_withdraw(&BOB::get(), 1, WithdrawReasons::TRANSFER, 49).is_err());
	});
}

#[test]
fn vested_transfer_fails_if_zero_period_or_count() {
	ExtBuilder::build().execute_with(|| {
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ADMIN::get()),
				BOB::get(),
				VestingBucket::Team,
				1_u64,
				100 * DOLLARS
			),
			Error::<Runtime>::ZeroVestingPeriod
		);
		assert_noop!(
			Vesting::vested_transfer(
				Origin::signed(ALICE::get()),
				BOB::get(),
				VestingBucket::Team,
				1u64,
				100 * DOLLARS
			),
			Error::<Runtime>::ZeroVestingPeriodCount
		);
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
				100 * DOLLARS
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
				1u64,
				u128::MAX
			),
			Error::<Runtime>::NumOverflow
		);
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
				VestingBucket::Team,
				0u64,
				100 * DOLLARS
			),
			BadOrigin
		);
	});
}

#[test]
fn claim_works() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			20 * DOLLARS
		));

		System::set_block_number(11);
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

		System::set_block_number(21);
		// claim more
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_ok!(PalletBalances::transfer(
			Origin::signed(BOB::get()),
			ALICE::get(),
			10 * DOLLARS
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
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			bucket: VestingBucket::Team,
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10 * DOLLARS,
		};
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			20 * DOLLARS
		));
		let schedule2 = VestingSchedule {
			bucket: VestingBucket::Team,
			start: 0u64,
			period: 10u64,
			period_count: 3u32,
			per_period: 10 * DOLLARS,
		};
		assert_ok!(Vesting::vested_transfer(
			Origin::signed(ADMIN::get()),
			BOB::get(),
			VestingBucket::Team,
			0u64,
			30 * DOLLARS
		));

		// There are 2 active vesting schedules for BOB::get()
		assert_eq!(
			Vesting::vesting_schedules(&BOB::get()),
			vec![schedule.clone(), schedule2.clone()]
		);

		// BOB::get() should receive 50 tokens at the end of all schedules
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), Balance::zero());

		// Should be usable first 20 tokens. 10 from each schedule
		System::set_block_number(11);
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 20 * DOLLARS);

		// There are 2 active vesting schedules
		assert_eq!(
			Vesting::vesting_schedules(&BOB::get()),
			vec![schedule, schedule2.clone()]
		);

		System::set_block_number(21);

		// First schedule is over. Plus 20 tokens. ( 10 from each schedule )
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(Vesting::vesting_schedules(&BOB::get()), vec![schedule2]);
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 40 * DOLLARS);

		System::set_block_number(31);

		// All schedules are finished. All tokens are usable
		assert_ok!(Vesting::claim(Origin::signed(BOB::get())));
		assert_eq!(PalletBalances::free_balance(BOB::get()), 50 * DOLLARS);
		assert_eq!(PalletBalances::usable_balance(BOB::get()), 50 * DOLLARS);
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
	assert_eq!(schedule1.per_period, 47_564_687_975); // 1 MNT / 21_024_000 blocks ~ 0,0000000476

	let schedule2: VestingSchedule<BlockNumber> = VestingSchedule::new(VestingBucket::Team, 100_000 * DOLLARS);
	assert_eq!(schedule2.bucket, VestingBucket::Team);
	assert_eq!(schedule2.start, 2_620_800);
	assert_eq!(schedule2.period_count as u128, 5 * BLOCKS_PER_YEAR);
	assert_eq!(schedule2.period, 1_u32);
	assert_eq!(schedule2.per_period, 3_805_175_038_051_750); // 100_000 MNT / 26_280_000 blocks ~ 0,0038

	let schedule3: VestingSchedule<BlockNumber> =
		VestingSchedule::new_beginning_from(VestingBucket::Marketing, 1234, 10 * DOLLARS);
	assert_eq!(schedule3.bucket, VestingBucket::Marketing);
	assert_eq!(schedule3.start, 1234);
	assert_eq!(schedule3.period_count as u128, BLOCKS_PER_YEAR);
	assert_eq!(schedule3.period, 1_u32);
	assert_eq!(schedule3.per_period, 1_902_587_519_025); // 1 MNT / 5256000 blocks ~ 0,00000019

	let schedule4: VestingSchedule<BlockNumber> =
		VestingSchedule::new_beginning_from(VestingBucket::StrategicPartners, 5000, 20 * DOLLARS);
	assert_eq!(schedule4.bucket, VestingBucket::StrategicPartners);
	assert_eq!(schedule4.start, 5000);
	assert_eq!(schedule4.period_count as u128, 2 * BLOCKS_PER_YEAR);
	assert_eq!(schedule4.period, 1_u32);
	assert_eq!(schedule4.per_period, 190_258_751_902_587_519_u128); // 20 MNT / 10512000 blocks ~ 0,0000019
}
