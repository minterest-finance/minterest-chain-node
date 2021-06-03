#![cfg(test)]

//! Unit tests for the genesis resources data.

use crate::chain_spec::{calculate_initial_allocations, calculate_vesting_list};
use frame_benchmarking::frame_support::sp_io;
use minterest_primitives::{AccountId, Balance, VestingBucket, VestingScheduleJson};
use node_minterest_runtime::{get_all_modules_accounts, MntTokenModuleId};
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::AccountIdConversion;
use std::collections::HashMap;

// Check the order of accounts. The mnt-token pallet must be placed first.
#[test]
fn get_all_modules_accounts_should_work() {
	assert_eq!(
		get_all_modules_accounts()[0],
		node_minterest_runtime::MntTokenModuleId::get().into_account()
	);
	assert_eq!(
		get_all_modules_accounts()[1],
		node_minterest_runtime::LiquidationPoolsModuleId::get().into_account()
	);
	assert_eq!(
		get_all_modules_accounts()[2],
		node_minterest_runtime::DexModuleId::get().into_account()
	);
	assert_eq!(
		get_all_modules_accounts()[3],
		node_minterest_runtime::LiquidityPoolsModuleId::get().into_account()
	);
}

#[test]
fn check_vesting_buckets_balances() {}

// Checks for the existence of a json file with initial token allocations.
// Checks the amounts of allocations and vesting.
#[test]
fn check_minterest_allocation_and_vesting() {
	sp_io::TestExternalities::default().execute_with(|| {
		let endowed_accounts = vec![AccountId::from([1u8; 32]), AccountId::from([2u8; 32])];
		let allocated_accounts_json = &include_bytes!("../../resources/dev-minterest-allocation-MNT.json")[..];
		let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
			serde_json::from_slice(allocated_accounts_json).unwrap();
		let allocated_list = allocated_list_parsed
			.iter()
			.flat_map(|(_bucket, schedules)| {
				schedules
					.iter()
					.map(|schedule| (schedule.account.clone(), schedule.amount))
			})
			.collect::<Vec<(AccountId, Balance)>>();
		let _ = calculate_initial_allocations(endowed_accounts, allocated_list);
		let _ = calculate_vesting_list(allocated_list_parsed);
	});
}

#[test]
#[should_panic(expected = "The total number of buckets in the allocation json file must be seven, but passed: 6")]
fn calculate_vesting_list_should_panic_if_missed_bucket() {
	sp_io::TestExternalities::default().execute_with(|| {
		let allocated_accounts_json = r#"{
			  "PrivateSale": [
				{
				  "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
				  "amount": 10001000000000000000000000
				}],
			  "PublicSale": [
				{
				  "account": "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
				  "amount": 2500250000000000000000000
				}],
			  "MarketMaking": [
				{
				  "account": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
				  "amount": 3000000000000000000000000
				}],
			  "StrategicPartners": [
				{
				  "account": "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
				  "amount": 1949100000000000000000000
				}],
			  "Marketing": [
				{
				  "account": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
				  "amount": 4000400000000000000000000
				}],
			  "Ecosystem": [
				{
				  "account": "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
				  "amount": 4499880000000000000000000
				}]
		}"#;
		let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
			serde_json::from_str(allocated_accounts_json).unwrap();
		let _ = calculate_vesting_list(allocated_list_parsed);
	});
}

// Bucket Market Making has wrong number of tokens - 6000000 MNT instead 3000000 MNT
#[test]
#[should_panic(
	expected = "The total amount of distributed tokens must be equal to the number of tokens in the bucket."
)]
fn calculate_vesting_list_should_panic_if_incorrect_account_balance() {
	sp_io::TestExternalities::default().execute_with(|| {
		let allocated_accounts_json = r#"{
			  "PrivateSale": [
				{
				  "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
				  "amount": 10001000000000000000000000
				}],
			  "PublicSale": [
				{
				  "account": "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
				  "amount": 2500250000000000000000000
				}],
			  "MarketMaking": [
				{
				  "account": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
				  "amount": 6000000000000000000000000
				}],
			  "StrategicPartners": [
				{
				  "account": "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
				  "amount": 1949100000000000000000000
				}],
			  "Marketing": [
				{
				  "account": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
				  "amount": 4000400000000000000000000
				}],
			  "Ecosystem": [
				{
				  "account": "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
				  "amount": 4499880000000000000000000
				}],
				"Team":
				[
				  {
					"account": "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
					"amount": 14017000000000000000000000
				  },
				  {
					"account": "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
					"amount": 10000000000000000000000000
				  }
				]
		}"#;
		let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
			serde_json::from_str(allocated_accounts_json).unwrap();
		let _ = calculate_vesting_list(allocated_list_parsed);
	});
}

// Account 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY occurs twice.
#[test]
#[should_panic(expected = "duplicate vesting accounts in genesis.")]
fn calculate_vesting_list_should_panic_if_duplicate_vesting_accounts() {
	sp_io::TestExternalities::default().execute_with(|| {
		let allocated_accounts_json = r#"{
			  "PrivateSale": [
				{
				  "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
				  "amount": 5001000000000000000000000
				},
				{
				  "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
				  "amount": 5000000000000000000000000
				}],
			  "PublicSale": [
				{
				  "account": "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
				  "amount": 2500250000000000000000000
				}],
			  "MarketMaking": [
				{
				  "account": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
				  "amount": 3000000000000000000000000
				}],
			  "StrategicPartners": [
				{
				  "account": "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
				  "amount": 1949100000000000000000000
				}],
			  "Marketing": [
				{
				  "account": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
				  "amount": 4000400000000000000000000
				}],
			  "Ecosystem": [
				{
				  "account": "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
				  "amount": 4499880000000000000000000
				}],
				"Team":
				[
				  {
					"account": "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
					"amount": 14017000000000000000000000
				  },
				  {
					"account": "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
					"amount": 10000000000000000000000000
				  }
				]
		}"#;
		let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
			serde_json::from_str(allocated_accounts_json).unwrap();
		let _ = calculate_vesting_list(allocated_list_parsed);
	});
}

#[test]
fn calculate_vesting_list_should_work() {
	sp_io::TestExternalities::default().execute_with(|| {
		let allocated_accounts_json = r#"{
			  "PrivateSale": [
				{
				  "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
				  "amount": 10001000000000000000000000
				}],
			  "PublicSale": [
				{
				  "account": "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
				  "amount": 2500250000000000000000000
				}],
			  "MarketMaking": [
				{
				  "account": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
				  "amount": 3000000000000000000000000
				}],
			  "StrategicPartners": [
				{
				  "account": "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
				  "amount": 1949100000000000000000000
				}],
			  "Marketing": [
				{
				  "account": "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
				  "amount": 4000400000000000000000000
				}],
			  "Ecosystem": [
				{
				  "account": "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
				  "amount": 4499880000000000000000000
				}],
				"Team":
				[
				  {
					"account": "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
					"amount": 14017000000000000000000000
				  },
				  {
					"account": "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
					"amount": 10000000000000000000000000
				  }
				]
		}"#;
		let allocated_list_parsed: HashMap<VestingBucket, Vec<VestingScheduleJson<AccountId, Balance>>> =
			serde_json::from_str(allocated_accounts_json).unwrap();
		let vesting_list = calculate_vesting_list(allocated_list_parsed);

		// Checking the vesting schedule for a Team Bucket member:
		assert_eq!(
			vesting_list
				.iter()
				.find(|(schedule, account, _, _, _, _)| schedule == &VestingBucket::Team
					&& account == &AccountId::from_string("5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw").unwrap()),
			Some(&(
				VestingBucket::Team,
				AccountId::from_string("5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw").unwrap(),
				182_u32,                 // half a year
				1_u32,                   // block by block
				26280000_u32,            // 5years * 5256000 = 26280000
				533371385083713850_u128  // 14017000000000000000000000 / 26280000 = 533371385083713850
			))
		);

		// Checking the vesting schedule for a Market Making member:
		assert_eq!(
			vesting_list
				.iter()
				.find(
					|(schedule, account, _, _, _, _)| schedule == &VestingBucket::MarketMaking
						&& account
							== &AccountId::from_string("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty").unwrap()
				)
				.unwrap(),
			&(
				VestingBucket::MarketMaking,
				AccountId::from_string("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty").unwrap(),
				0_u32,                          // from the start of the protocol
				1_u32,                          // block by block
				0_u32,                          // user will receive all tokens in block 0. The lock will not be established
				3000000000000000000000000_u128  // all 30 millions MNT tokens
			)
		)
	});
}

#[test]
fn calculate_initial_allocations_should_work() {
	let endowed_accounts = vec![AccountId::from([1u8; 32]), AccountId::from([2u8; 32])];
	let allocated_list = vec![
		(AccountId::from([1u8; 32]), 19967630000000000000000000_u128), // 19,967,630 MNT
		(AccountId::from([3u8; 32]), 10000000000000000000000000_u128), // 10,000,000 MNT
		(AccountId::from([4u8; 32]), 10000000000000000000000000_u128), // 10,000,000 MNT
		(AccountId::from([5u8; 32]), 10000000000000000000000000_u128), // 10,000,000 MNT
	];
	let initial_allocations = calculate_initial_allocations(endowed_accounts, allocated_list);

	// Initial allocation for the first account equal `initial_allocation + ED`:
	// 19,967,630 + 1 = 19,967,631 MNT
	assert_eq!(
		initial_allocations
			.iter()
			.find(|(account_id, _)| account_id == &AccountId::from([1u8; 32]))
			.unwrap(),
		&(AccountId::from([1u8; 32]), 19967631000000000000000000_u128)
	);

	// Initial allocation for the mnt_token pallet equal `50,032,400 - sum(ED)`:
	// 50,032,400 - 2 = 50,032,398 MNT
	assert_eq!(
		initial_allocations
			.iter()
			.find(|(account_id, _)| account_id == &MntTokenModuleId::get().into_account())
			.unwrap(),
		&(MntTokenModuleId::get().into_account(), 50032398000000000000000000_u128)
	);
}

#[test]
#[should_panic(expected = "Total allocation must be equal to 100,000,030 MNT tokens, but passed: 110000030 MNT")]
fn calculate_initial_allocations_should_panic_incorrect_sum_allocation() {
	let endowed_accounts = vec![AccountId::from([1u8; 32]), AccountId::from([2u8; 32])];
	let allocated_list = vec![
		(AccountId::from([1u8; 32]), 19967630000000000000000000_u128), // 19,967,630 MNT
		(AccountId::from([3u8; 32]), 20000000000000000000000000_u128), // 20,000,000 MNT
		(AccountId::from([4u8; 32]), 10000000000000000000000000_u128), // 10,000,000 MNT
		(AccountId::from([5u8; 32]), 10000000000000000000000000_u128), // 10,000,000 MNT
	];
	let _ = calculate_initial_allocations(endowed_accounts, allocated_list);
}
