use crate::{benchmarking::utils::SEED, AccountId, MaxMembersWhitelistMode, Runtime, Whitelist};
use frame_benchmarking::account;
use frame_support::{assert_ok, traits::EnsureOrigin};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

runtime_benchmarks! {
	{Runtime, whitelist_module}

	_ {}

	add_member {
		let m in 1 .. MaxMembersWhitelistMode::get() as u32 - 1_u32;

		let mut members = (0..m).map(|i| account("member", i, SEED)).collect::<Vec<AccountId>>();
		members.sort();
		whitelist_module::Members::<Runtime>::set(members.clone());
		let new_member = account::<AccountId>("add", m, SEED);
	}: {
		assert_ok!(Whitelist::add_member(<Runtime as whitelist_module::Config>::WhitelistOrigin::successful_origin(), new_member.clone()));
	}
	verify {
		assert!(Whitelist::members().contains(&new_member));
	}

	remove_member {
		let m in 2 .. MaxMembersWhitelistMode::get() as u32 - 1_u32;
		let mut members = (0..m).map(|i| account("member", i, SEED)).collect::<Vec<AccountId>>();
		members.sort();
		whitelist_module::Members::<Runtime>::set(members.clone());
		let to_remove = members.first().cloned().unwrap();
	}: {
		assert_ok!(Whitelist::remove_member(<Runtime as whitelist_module::Config>::WhitelistOrigin::successful_origin(), to_remove.clone()));
	}
	verify {
		assert!(!Whitelist::members().contains(&to_remove));
	}


	switch_whitelist_mode {}: _(
		RawOrigin::Root
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::test_externalities;
	use frame_support::assert_ok;

	#[test]
	fn test_add_member() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_add_member());
		})
	}

	#[test]
	fn test_remove_member() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_remove_member());
		})
	}

	#[test]
	fn test_switch_whitelist_mode() {
		test_externalities().execute_with(|| {
			assert_ok!(test_benchmark_switch_whitelist_mode());
		})
	}
}
