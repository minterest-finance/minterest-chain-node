//! Tests for the accounts pallet.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn add_member_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		// The dispatch origin of this call must be _Root_.
		assert_noop!(TestAccounts::add_member(Origin::signed(ALICE), BOB), BadOrigin);

		// Add Alice to allow-list.
		assert_ok!(TestAccounts::add_member(Origin::root(), ALICE));
		let expected_event = Event::test_accounts(crate::Event::AccountAdded(ALICE));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(<AllowedAccounts<Test>>::contains_key(ALICE));

		// Add Bob to allow-list.
		assert_ok!(TestAccounts::add_member(Origin::root(), BOB));
		let expected_event = Event::test_accounts(crate::Event::AccountAdded(BOB));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(<AllowedAccounts<Test>>::contains_key(BOB));

		// Alice cannot be added to the allowed list because she has already been added.
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
		// Add Alice to allow-list.
		assert_ok!(TestAccounts::add_member(Origin::root(), ALICE));
		let expected_event = Event::test_accounts(crate::Event::AccountAdded(1));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Add Bob to allow-list.
		assert_ok!(TestAccounts::add_member(Origin::root(), BOB));
		let expected_event = Event::test_accounts(crate::Event::AccountAdded(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Remove Bob from allow-list.
		assert_ok!(TestAccounts::remove_member(Origin::root(), BOB));
		let expected_event = Event::test_accounts(crate::Event::AccountRemoved(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Cannot remove Alice, because ay least one member must remain.
		assert_noop!(
			TestAccounts::remove_member(Origin::root(), ALICE),
			Error::<Test>::MustBeAtLeastOneMember
		);

		// Check storage changes.
		assert!(<AllowedAccounts<Test>>::contains_key(ALICE));
		assert!(!<AllowedAccounts<Test>>::contains_key(BOB));

		// Bob was previously removed from the allow-list.
		assert_noop!(
			TestAccounts::remove_member(Origin::root(), BOB),
			Error::<Test>::NotAnAdmin
		);
	})
}

#[test]
fn is_admin_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		// Add Alice to allow-list.
		assert_ok!(TestAccounts::add_member(Origin::root(), ALICE));

		assert_ok!(TestAccounts::is_admin(Origin::signed(ALICE)));
		let expected_event = Event::test_accounts(crate::Event::IsAnAdmin(ALICE));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		assert_noop!(TestAccounts::is_admin(Origin::signed(BOB)), Error::<Test>::NotAnAdmin);
	});
}

#[test]
fn is_admin_internal_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		assert_ok!(TestAccounts::add_member(Origin::root(), ALICE));

		assert!(TestAccounts::is_admin_internal(&ALICE));
		assert!(!TestAccounts::is_admin_internal(&BOB));
	});
}
