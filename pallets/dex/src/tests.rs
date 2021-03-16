//! Unit tests for dex module.

#![cfg(test)]

use crate::mock::*;

#[test]
fn set_dummy_work() {
	new_test_ext().execute_with(|| {
		assert!(true);
	});
}
