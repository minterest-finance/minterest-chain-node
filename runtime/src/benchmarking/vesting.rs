use super::utils::{lookup_of_account, set_balance};
use crate::{
	AccountId, Currencies, MaxVestingSchedules, MinVestedTransfer, Runtime, System, Vesting, BLOCKS_PER_YEAR, DOLLARS,
	MNT,
};
use core::convert::TryInto;
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

	_ {}

	claim {
		// The number of schedules on the account is set equal MaxVestingSchedules.
		let i in 1 .. MaxVestingSchedules::get();

		// The number of MNT tokens that are transferred according to the schedule.
		// Doubled due to math problems (total_amount = 0.999 < MinVestedTransfer = 1.0)
		let schedule_amount = 2_u128 * MinVestedTransfer::get();
		let mut schedule = VestingSchedule::new(VestingBucket::Team, schedule_amount);

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
		// The number of MNT tokens that are transferred according to the schedule.
		// Doubled due to math problems (total_amount = 0.999 < MinVestedTransfer = 1.0)
		let schedule_amount = 2_u128 * MinVestedTransfer::get();

		set_balance(MNT, &VestingBucket::Team.bucket_account_id().unwrap(), schedule_amount)?;

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = lookup_of_account(to.clone());
	}: _(RawOrigin::Root, to_lookup.clone(), VestingBucket::Team, 0, schedule_amount)
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::total_balance(MNT, &to),
			schedule_amount - 1_u128
		);
	}

	remove_vesting_schedules {
		let schedule_amount = 100 * DOLLARS; // 100 MNT
		set_balance(MNT, &VestingBucket::Team.bucket_account_id().unwrap(), schedule_amount * MaxVestingSchedules::get() as u128)?;

		let teammate: AccountId = account("teammate", 0, SEED);
		let teammate_lookup = lookup_of_account(teammate.clone());

		for _ in 0..MaxVestingSchedules::get() {
			Vesting::vested_transfer(RawOrigin::Root.into(), teammate_lookup.clone(), VestingBucket::Team, 0, schedule_amount)?;
		}
		System::set_block_number((BLOCKS_PER_YEAR * 5 / 2).try_into().unwrap());

	}: _(RawOrigin::Root, teammate_lookup.clone(), VestingBucket::Team)
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::free_balance(MNT, &teammate),
			schedule_amount,
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
