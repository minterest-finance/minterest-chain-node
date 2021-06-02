#![cfg(test)]

//! Unit tests for the genesis resources data.

use minterest_primitives::{AccountId, Balance, BlockNumber, VestingBucket, VestingScheduleJson};
use node_minterest_runtime::BLOCKS_PER_YEAR;
use sp_runtime::traits::One;
use std::collections::HashMap;

#[test]
fn check_minterest_vesting_unique_accounts_and_buckets_balance() {
	let allocated_accounts_json = &include_bytes!("../../resources/dev-minterest-allocation-MNT.json")[..];
	let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
		serde_json::from_slice(allocated_accounts_json).unwrap();

	let mut vesting_list: Vec<(VestingBucket, AccountId, BlockNumber, BlockNumber, u32, Balance)> = Vec::new();

	for (bucket, schedules) in allocated_list_parsed.iter() {
		let total_bucket_amount: Balance = schedules.iter().map(|schedule| schedule.amount).sum();
		assert_eq!(
			total_bucket_amount,
			bucket.total_amount(),
			"total amount of distributed tokens must be equal to the number of tokens in the bucket."
		);

		for schedule in schedules.iter() {
			let start: BlockNumber = bucket.unlock_begins_in_days().into();
			let period: BlockNumber = BlockNumber::one(); // block by block

			let period_count: u32 = bucket.vesting_duration() as u32 * BLOCKS_PER_YEAR as u32;

			let per_period: Balance = schedule
				.amount
				.checked_div(period_count as u128)
				.unwrap_or(schedule.amount);

			vesting_list.push((
				bucket.clone(),
				schedule.account.clone(),
				start,
				period,
				period_count,
				per_period,
			));
		}
	}

	// ensure no duplicates exist.
	let unique_vesting_accounts = vesting_list
		.iter()
		.map(|(_, account, _, _, _, _)| account)
		.cloned()
		.collect::<std::collections::BTreeSet<_>>();

	assert_eq!(unique_vesting_accounts.len(), vesting_list.len());
}
