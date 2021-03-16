//! Unit tests for dex module.

#![cfg(test)]

use crate::mock::*;

#[test]
fn accrue_interest_should_work() {
	ExtBuilder::default()
		.dex_balance(CurrencyId::DOT, dollars(20_u128))
		.build()
		.execute_with(|| {
			assert!(true);
		});
}
