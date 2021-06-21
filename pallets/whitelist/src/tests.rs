//! Tests for the whitelist module.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok, error::BadOrigin};

#[test]
fn add_member_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(Whitelist::add_member(Origin::signed(ALICE), BOB), BadOrigin);

		// Add Alice to whitelist.
		assert_ok!(Whitelist::add_member(Origin::signed(ADMIN), ALICE));
		let expected_event = Event::whitelist(crate::Event::MemberAdded(ALICE));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(Members::<Test>::get().contains(&ALICE));

		// Add Bob to whitelist.
		assert_ok!(Whitelist::add_member(Origin::signed(ADMIN), BOB));
		let expected_event = Event::whitelist(crate::Event::MemberAdded(BOB));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(Members::<Test>::get().contains(&BOB));

		// Alice cannot be added to the whitelist because she has already been added.
		assert_noop!(
			Whitelist::add_member(Origin::signed(ADMIN), ALICE),
			Error::<Test>::AlreadyMember
		);
	});
}

#[test]
fn cant_exceed_max_members() {
	ExternalityBuilder::build().execute_with(|| {
		// Add 16 members, reaching the max.
		for i in 0..16 {
			assert_ok!(Whitelist::add_member(Origin::signed(ADMIN), i));
		}

		// Try to add the 17th member exceeding the max.
		assert_noop!(
			Whitelist::add_member(Origin::signed(ADMIN), 16),
			Error::<Test>::MembershipLimitReached
		);
	})
}

#[test]
fn remove_member_should_work() {
	ExternalityBuilder::build().execute_with(|| {
		// Add Alice to whitelist.
		assert_ok!(Whitelist::add_member(Origin::signed(ADMIN), ALICE));
		let expected_event = Event::whitelist(crate::Event::MemberAdded(1));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Add Bob to whitelist.
		assert_ok!(Whitelist::add_member(Origin::signed(ADMIN), BOB));
		let expected_event = Event::whitelist(crate::Event::MemberAdded(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Add and remove CHARLIE from whitelist.
		assert_ok!(Whitelist::add_member(Origin::signed(ADMIN), CHARLIE));
		assert_ok!(Whitelist::remove_member(Origin::signed(ADMIN), CHARLIE));
		// Charlie was previously removed from the whitelist.
		assert_noop!(
			Whitelist::remove_member(Origin::signed(ADMIN), CHARLIE),
			Error::<Test>::NotMember
		);

		// Remove Bob from whitelist.
		assert_ok!(Whitelist::remove_member(Origin::signed(ADMIN), BOB));
		let expected_event = Event::whitelist(crate::Event::MemberRemoved(2));
		assert!(System::events().iter().any(|record| record.event == expected_event));

		// Cannot remove Alice, because at least one member must remain.
		assert_noop!(
			Whitelist::remove_member(Origin::signed(ADMIN), ALICE),
			Error::<Test>::MustBeAtLeastOneMember
		);

		// Check storage changes.
		assert!(Members::<Test>::get().contains(&ALICE));
		assert!(!Members::<Test>::get().contains(&BOB));
	})
}
