//! Unit tests for example module.

#![cfg(test)]

use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn set_dummy_work() {
	test_externalities().execute_with(|| {});
}

#[test]
fn do_set_bar_work() {
	test_externalities().execute_with(|| {});
}
