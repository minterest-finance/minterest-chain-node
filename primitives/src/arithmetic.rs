use crate::*;
use sp_runtime::{traits::CheckedMul, DispatchError, FixedPointNumber};

/// Performs mathematical calculations.
///
/// returns `value = balance_value + balance_scalar * rate_scalar`
pub fn checked_acc_and_add_mul(
	balance_value: Balance,
	balance_scalar: Balance,
	rate_scalar: Rate,
) -> sp_std::result::Result<Balance, DispatchError> {
	let value = Rate::from_inner(balance_scalar)
		.checked_mul(&rate_scalar)
		.map(|x| x.into_inner())
		.and_then(|v| v.checked_add(balance_value))
		.ok_or(DispatchError::Other("Overflow Error"))?;
	Ok(value)
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_err;

	#[test]
	fn checked_acc_and_add_mul_should_work() {
		// 20 + 20 * 0.9 = 38
		assert_eq!(
			checked_acc_and_add_mul(20, 20, Rate::saturating_from_rational(9, 10)),
			Ok(38)
		);
		// 120_000 + 85_000 * 0.87 = 193_950
		assert_eq!(
			checked_acc_and_add_mul(120_000, 85_000, Rate::saturating_from_rational(87, 100)),
			Ok(193950)
		);

		// Overflow in calculation: max_value() * 1.9
		assert_err!(
			checked_acc_and_add_mul(100, Balance::MAX, Rate::saturating_from_rational(19, 10)),
			DispatchError::Other("Overflow Error")
		);

		// Overflow in calculation: max_value() + 100 * 1.9
		assert_err!(
			checked_acc_and_add_mul(Balance::MAX, 100, Rate::saturating_from_rational(19, 10)),
			DispatchError::Other("Overflow Error")
		);
	}
}
