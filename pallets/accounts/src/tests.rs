//! Tests for the accounts pallet.

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn add_member_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(TestAccounts::add_member(Origin::signed(ALICE), BOB), BadOrigin);

		assert_ok!(TestAccounts::add_member(Origin::root(), ALICE));
		let expected_event = TestEvent::accounts(RawEvent::AccountAdded(ALICE));
		assert_eq!(System::events()[0].event, expected_event,);
		assert!(<AllowedAccounts<Test>>::contains_key(ALICE));

		assert_ok!(TestAccounts::add_member(Origin::root(), BOB));
		assert_noop!(
			TestAccounts::add_member(Origin::root(), ALICE),
			Error::<Test>::AlreadyMember
		);
	});
}

#[test]
fn cant_exceed_max_members() {
	ExternalityBuilder::build().execute_with(|| {
		// Add 16 members, reaching the max
		for i in 0..16 {
			assert_ok!(TestAccounts::add_member(Origin::root(), i));
		}

		// Try to add the 17th member exceeding the max
		assert_noop!(
			TestAccounts::add_member(Origin::root(), 16),
			Error::<Test>::MembershipLimitReached
		);
	})
}

#[test]
fn remove_member_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		assert_ok!(TestAccounts::add_member(Origin::root(), ALICE));
		assert_ok!(TestAccounts::remove_member(Origin::root(), ALICE));

		// check correct event emission
		let expected_event = TestEvent::accounts(RawEvent::AccountRemoved(ALICE));

		assert_eq!(System::events()[1].event, expected_event,);

		// check storage changes
		assert!(!<AllowedAccounts<Test>>::contains_key(ALICE));

		assert_noop!(
			TestAccounts::remove_member(Origin::root(), BOB),
			Error::<Test>::NotMember
		);
	})
}
