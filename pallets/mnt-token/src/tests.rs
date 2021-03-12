#![cfg(test)]

use crate::mock::*;
use frame_support::assert_ok;
use minterest_primitives::Rate;
use sp_arithmetic::FixedPointNumber;

#[test]
fn test_set_mnt_rate() {
	new_test_ext().execute_with(|| {
		let old_rate = Rate::zero();
		let new_rate = Rate::saturating_from_rational(11, 10);
		assert_eq!(MntToken::mnt_rate(), old_rate);
		assert_ok!(MntToken::set_mnt_rate(Origin::root(), new_rate));
		assert_eq!(MntToken::mnt_rate(), new_rate);
		let new_mnt_rate_event = Event::mnt_token(crate::Event::NewMntRate(old_rate, new_rate));
		assert!(System::events().iter().any(|record| record.event == new_mnt_rate_event));
	});
}
