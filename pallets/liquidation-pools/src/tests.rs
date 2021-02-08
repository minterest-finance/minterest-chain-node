//! Tests for the liquidation-pools pallet.

use super::*;
use mock::*;

#[test]
fn add_member_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		assert!(true);
	});
}
