//! Unit tests for dex module.

#![cfg(test)]

use crate::mock::*;

#[test]
fn do_swap_with_exact_target_should_work() {
	ExtBuilder::default()
		.dex_balance(DOT, dollars(10_u128))
		.build()
		.execute_with(|| {
			assert_eq!(TestDex::get_dex_available_liquidity(DOT), dollars(10_u128));
		});
}
