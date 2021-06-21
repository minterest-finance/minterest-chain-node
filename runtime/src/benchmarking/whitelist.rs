use crate::{
	benchmarking::utils::{set_whitelist_members, SEED},
	AccountId, MaxMembersWhitelistMode, Runtime, Whitelist,
};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use sp_std::prelude::*;

runtime_benchmarks! {
	{Runtime, whitelist}

	_ {}

	add_member {
		let m in 1 .. MaxMembersWhitelistMode::get() as u32 - 1_u32;

		let members = (0..m).map(|i| account("member", i, SEED)).collect::<Vec<AccountId>>();
		// whitelist::Members::<Runtime>::put(members.clone());
		set_whitelist_members(members.clone())?;

		let new_member = account::<AccountId>("add", m, SEED);
	}: _(RawOrigin::Root, new_member.clone())
	verify {
		assert!(Whitelist::members().contains(&new_member));
		whitelist::Members::<Runtime>::set(vec![]);
	}

	remove_member {
		let m in 2 .. MaxMembersWhitelistMode::get() as u32 - 1_u32;

		let members = (0..m).map(|i| account("member", i, SEED)).collect::<Vec<AccountId>>();
		// whitelist::Members::<Runtime>::put(members.clone());
		set_whitelist_members(members.clone())?;

		let to_remove = members.first().cloned().unwrap();
	}: _(RawOrigin::Root, to_remove.clone())
	verify {
		assert!(!Whitelist::members().contains(&to_remove));
		whitelist::Members::<Runtime>::set(vec![]);
	}
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
}
