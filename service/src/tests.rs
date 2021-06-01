#![cfg(test)]

//! Unit tests for the genesis resources data.

use minterest_primitives::{AccountId, Balance, BlockNumber, VestingBucket, VestingScheduleJson};
use std::collections::HashMap;

#[test]
fn check_minterest_vesting_unique_accounts() {
	let vesting_json = &include_bytes!("../../resources/dev-minterest-allocation-MNT.json")[..];

	let vesting_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
		serde_json::from_slice(vesting_json).unwrap();

	let mut vesting: Vec<(VestingBucket, AccountId, BlockNumber, BlockNumber, u32, Balance)> = Vec::new();

	for (bucket, schedules) in vesting_parsed.iter() {
		for schedule in schedules.iter() {
			vesting.push((
				bucket.clone(),
				schedule.account.clone(),
				schedule.start,
				schedule.period,
				schedule.period_count,
				schedule.per_period,
			));
		}
	}

	// ensure no duplicates exist.
	let unique_dev_vesting_accounts = vesting
		.iter()
		.map(|(_, account, _, _, _, _)| account)
		.cloned()
		.collect::<std::collections::BTreeSet<_>>();

	assert_eq!(unique_dev_vesting_accounts.len(), vesting.len(),);
}
