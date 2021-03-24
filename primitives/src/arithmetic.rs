use crate::*;
use sp_runtime::{traits::CheckedMul, FixedPointNumber};

/// Performs mathematical calculations.
///
/// returns `value = balance_value + balance_scalar * rate_scalar`
pub fn checked_acc_and_add_mul(balance_value: Balance, balance_scalar: Balance, rate_scalar: Rate) -> Option<Balance> {
	Rate::from_inner(balance_scalar)
		.checked_mul(&rate_scalar)
		.map(|x| x.into_inner())
		.and_then(|v| v.checked_add(balance_value))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn checked_acc_and_add_mul_should_work() {
		// 20 + 20 * 0.9 = 38
		assert_eq!(
			checked_acc_and_add_mul(20, 20, Rate::saturating_from_rational(9, 10)),
			Some(38)
		);
		// 120_000 + 85_000 * 0.87 = 193_950
		assert_eq!(
			checked_acc_and_add_mul(120_000, 85_000, Rate::saturating_from_rational(87, 100)),
			Some(193950)
		);

		// Overflow in calculation: max_value() * 1.9
		assert_eq!(
			checked_acc_and_add_mul(100, Balance::MAX, Rate::saturating_from_rational(19, 10)),
			None
		);

		// Overflow in calculation: max_value() + 100 * 1.9
		assert_eq!(
			checked_acc_and_add_mul(Balance::MAX, 100, Rate::saturating_from_rational(19, 10)),
			None
		);
	}
}
