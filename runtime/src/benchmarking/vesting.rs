use super::utils::{lookup_of_account, set_balance};
use crate::{
	AccountId, BlockNumber, Currencies, MaxVestingSchedules, Runtime, System, Vesting, BLOCKS_PER_YEAR, DOLLARS, MNT,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use minterest_primitives::VestingBucket;
use module_vesting::VestingSchedule;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use sp_std::prelude::*;

const SEED: u32 = 0;

runtime_benchmarks! {
	{ Runtime, module_vesting }

	claim {
		let i in 1 .. MaxVestingSchedules::get();

		let schedule_amount = 10 * DOLLARS; // 10 MNT
		let mut schedule: VestingSchedule<BlockNumber> = VestingSchedule::new(VestingBucket::Team, schedule_amount);

		set_balance(MNT, &VestingBucket::Team.bucket_account_id().unwrap(), schedule_amount * i as u128)?;

		let claimer: AccountId = account("claimer", 0, SEED);
		let claimer_lookup = lookup_of_account(claimer.clone());

		for _ in 0..i {
			schedule.start = i;
			Vesting::vested_transfer(RawOrigin::Root.into(), claimer_lookup.clone(), VestingBucket::Team, i, schedule_amount)?;
		}
		System::set_block_number(schedule.end().unwrap() + 1u32);
	}: _(RawOrigin::Signed(claimer.clone()))
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::free_balance(MNT, &claimer),
			schedule.total_amount().unwrap() * i as u128,
		);
	}

	vested_transfer {
		let schedule_amount = 10 * DOLLARS; // 10 MNT
		let schedule: VestingSchedule<BlockNumber> = VestingSchedule::new(VestingBucket::Team, schedule_amount);

		set_balance(MNT, &VestingBucket::Team.bucket_account_id().unwrap(), schedule_amount)?;

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = lookup_of_account(to.clone());
	}: _(RawOrigin::Root, to_lookup.clone(), VestingBucket::Team, 0, schedule_amount)
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::total_balance(MNT, &to),
			schedule.total_amount().unwrap()
		);
	}

	remove_vesting_schedules {
		let schedule_amount = 10 * DOLLARS; // 10 MNT

		set_balance(MNT, &VestingBucket::Team.bucket_account_id().unwrap(), schedule_amount * MaxVestingSchedules::get() as u128)?;

		let teammate: AccountId = account("teammate", 0, SEED);
		let teammate_lookup = lookup_of_account(teammate.clone());

		for _ in 0..MaxVestingSchedules::get() {
			Vesting::vested_transfer(RawOrigin::Root.into(), teammate_lookup.clone(), VestingBucket::Team, 0, schedule_amount)?;
		}

		System::set_block_number((VestingBucket::Team.vesting_duration() as u128 * BLOCKS_PER_YEAR / 2_u128) as u32);
	}: _(RawOrigin::Root, teammate_lookup.clone(), VestingBucket::Team)
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::free_balance(MNT, &teammate),
			schedule_amount * MaxVestingSchedules::get() as u128 / 2_u128,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_claim() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_claim());
		})
	}

	#[test]
	fn test_vested_transfer() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_vested_transfer());
		})
	}

	#[test]
	fn test_remove_vesting_schedules() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_remove_vesting_schedules());
		})
	}
}
