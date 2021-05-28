use super::utils::{lookup_of_account, set_aca_balance};
use crate::{
	AccountId, AccountIdConversion, Balance, BlockNumber, Currencies, KaruraFoundationAccounts, MaxVestingSchedules,
	MinVestedTransfer, Runtime, System, Vesting, KAR,
};

use sp_std::prelude::*;

use frame_benchmarking::{account, whitelisted_caller};
use frame_system::RawOrigin;

use module_vesting::VestingSchedule;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;

pub type Schedule = VestingSchedule<BlockNumber, Balance>;

const SEED: u32 = 0;

runtime_benchmarks! {
	{ Runtime, module_vesting }

	vested_transfer {
		let schedule = Schedule {
			start: 0,
			period: 2,
			period_count: 3,
			per_period: MinVestedTransfer::get(),
		};

		// extra 1 dollar to pay fees
		let from: AccountId = KaruraFoundationAccounts::get()[0].clone();
		set_aca_balance(&from, schedule.total_amount().unwrap() + dollar(KAR));

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = lookup_of_account(to.clone());
	}: _(RawOrigin::Signed(from), to_lookup, schedule.clone())
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::total_balance(KAR, &to),
			schedule.total_amount().unwrap()
		);
	}

	claim {
		let i in 1 .. MaxVestingSchedules::get();

		let mut schedule = Schedule {
			start: 0,
			period: 2,
			period_count: 3,
			per_period: MinVestedTransfer::get(),
		};

		let from: AccountId = KaruraFoundationAccounts::get()[0].clone();
		// extra 1 dollar to pay fees
		set_aca_balance(&from, schedule.total_amount().unwrap() * i as u128 + dollar(KAR));

		let to: AccountId = whitelisted_caller();
		let to_lookup = lookup_of_account(to.clone());

		for _ in 0..i {
			schedule.start = i;
			Vesting::vested_transfer(RawOrigin::Signed(from.clone()).into(), to_lookup.clone(), schedule.clone())?;
		}
		System::set_block_number(schedule.end().unwrap() + 1u32);
	}: _(RawOrigin::Signed(to.clone()))
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::free_balance(KAR, &to),
			schedule.total_amount().unwrap() * i as u128,
		);
	}

	update_vesting_schedules {
		let i in 1 .. MaxVestingSchedules::get();

		let mut schedule = Schedule {
			start: 0,
			period: 2,
			period_count: 3,
			per_period: MinVestedTransfer::get(),
		};

		let to: AccountId = account("to", 0, SEED);
		set_aca_balance(&to, schedule.total_amount().unwrap() * i as u128);
		let to_lookup = lookup_of_account(to.clone());

		let mut schedules = vec![];
		for _ in 0..i {
			schedule.start = i;
			schedules.push(schedule.clone());
		}
	}: _(RawOrigin::Root, to_lookup, schedules)
	verify {
		assert_eq!(
			<Currencies as MultiCurrency<_>>::free_balance(KAR, &to),
			schedule.total_amount().unwrap() * i as u128
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use orml_benchmarking::impl_benchmark_test_suite;

	impl_benchmark_test_suite!(new_test_ext(),);
}
