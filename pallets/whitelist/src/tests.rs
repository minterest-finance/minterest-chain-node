//! Tests for the whitelist module.

use super::*;
use mock::{Event, *};

use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use std::collections::BTreeSet;

#[test]
#[should_panic(expected = "Duplicate member account in whitelist in genesis.")]
fn genesis_duplicate_member_account_should_panic() {
	ExternalityBuilder::default()
		.set_members(vec![ALICE, BOB, BOB, CHARLIE, ADMIN])
		.build();
}

#[test]
#[should_panic(expected = "Exceeded the number of whitelist members in genesis.")]
fn genesis_exceeded_number_of_members_should_panic() {
	ExternalityBuilder::default()
		.set_members((0..100).collect::<Vec<AccountId>>())
		.build();
}

#[test]
fn query_membership_works() {
	ExternalityBuilder::default()
		.set_members(vec![ALICE, BOB, CHARLIE, ADMIN])
		.build()
		.execute_with(|| {
			// Sorted list.
			assert_eq!(
				Whitelist::get_whitelist_members(),
				vec![ADMIN, ALICE, BOB, CHARLIE]
					.into_iter()
					.collect::<BTreeSet<AccountId>>()
			);
		});
}

#[test]
fn add_member_should_works() {
	ExternalityBuilder::default().build().execute_with(|| {
		// The dispatch origin of this call must be Root or half MinterestCouncil.
		assert_noop!(Whitelist::add_member(Origin::signed(ALICE), BOB), BadOrigin);

		// Add Alice to whitelist.
		assert_ok!(Whitelist::add_member(admin_origin(), ALICE));
		let expected_event = Event::Whitelist(crate::Event::MemberAdded(ALICE));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(Members::<Test>::contains_key(&ALICE));

		// Add Bob to whitelist.
		assert_ok!(Whitelist::add_member(admin_origin(), BOB));
		let expected_event = Event::Whitelist(crate::Event::MemberAdded(BOB));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(Members::<Test>::contains_key(&BOB));

		// Alice cannot be added to the whitelist because she has already been added.
		assert_noop!(
			Whitelist::add_member(admin_origin(), ALICE),
			Error::<Test>::MemberAlreadyAdded
		);

		assert_eq!(
			Whitelist::get_whitelist_members(),
			vec![ALICE, BOB].into_iter().collect::<BTreeSet<AccountId>>()
		);
	});
}

#[test]
fn cant_exceed_max_members() {
	ExternalityBuilder::default().build().execute_with(|| {
		// Add 16 members, reaching the max.
		for i in (0..16).rev() {
			assert_ok!(Whitelist::add_member(admin_origin(), i));
		}

		// Try to add the 17th member exceeding the max.
		assert_noop!(
			Whitelist::add_member(admin_origin(), 16),
			Error::<Test>::MembershipLimitReached
		);

		// Sorted whitelist.
		assert_eq!(
			Whitelist::get_whitelist_members(),
			(0..16).collect::<BTreeSet<AccountId>>()
		);
	})
}

#[test]
fn remove_member_should_works() {
	ExternalityBuilder::default()
		.set_members(vec![ALICE, BOB, CHARLIE])
		.build()
		.execute_with(|| {
			assert_ok!(Whitelist::remove_member(admin_origin(), CHARLIE));
			// Charlie was previously removed from the whitelist.
			assert_noop!(
				Whitelist::remove_member(admin_origin(), CHARLIE),
				Error::<Test>::MemberNotExist
			);

			// Remove Bob from whitelist.
			assert_ok!(Whitelist::remove_member(admin_origin(), BOB));
			let expected_event = Event::Whitelist(crate::Event::MemberRemoved(2));
			assert!(System::events().iter().any(|record| record.event == expected_event));

			// Check storage changes.
			assert_eq!(
				Whitelist::get_whitelist_members(),
				vec![ALICE].into_iter().collect::<BTreeSet<AccountId>>()
			);
		})
}

#[test]
fn switch_whitelist_mode_should_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		assert_ok!(Whitelist::switch_whitelist_mode(admin_origin(), true));
		let expected_event = Event::Whitelist(crate::Event::ProtocolOperationModeSwitched(true));
		assert!(System::events().iter().any(|record| record.event == expected_event));
		assert!(Whitelist::whitelist_mode());

		assert_ok!(Whitelist::switch_whitelist_mode(admin_origin(), false));
		assert!(!Whitelist::whitelist_mode());

		assert_noop!(Whitelist::switch_whitelist_mode(alice_origin(), true), BadOrigin);
		assert!(!Whitelist::whitelist_mode());

		assert_noop!(
			Whitelist::switch_whitelist_mode(admin_origin(), false),
			Error::<Test>::ModeChangeError
		);
		assert!(!Whitelist::whitelist_mode());
	});
}
