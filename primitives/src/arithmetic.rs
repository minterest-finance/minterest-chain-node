use crate::*;
use sp_runtime::{traits::CheckedMul, DispatchError, FixedPointNumber};

/// Performs mathematical calculations.
///
/// returns `value = addendum + multiplier_one * multiplier_two`
pub fn sum_with_mult_result(
	addendum: Balance,
	multiplier_one: Balance,
	multiplier_two: Rate,
) -> sp_std::result::Result<Balance, DispatchError> {
	let value = Rate::from_inner(multiplier_one)
		.checked_mul(&multiplier_two)
		.map(|x| x.into_inner())
		.and_then(|v| v.checked_add(addendum))
		.ok_or(DispatchError::Other("Overflow Error"))?;
	Ok(value)
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_err;

	#[test]
	fn sum_with_mult_result_should_work() {
		// 20 + 20 * 0.9 = 38
		assert_eq!(
			sum_with_mult_result(20, 20, Rate::saturating_from_rational(9, 10)),
			Ok(38)
		);
		// 120_000 + 85_000 * 0.87 = 193_950
		assert_eq!(
			sum_with_mult_result(120_000, 85_000, Rate::saturating_from_rational(87, 100)),
			Ok(193950)
		);

		// Overflow in calculation: max_value() * 1.9
		assert_err!(
			sum_with_mult_result(100, Balance::MAX, Rate::saturating_from_rational(19, 10)),
			DispatchError::Other("Overflow Error")
		);

		// Overflow in calculation: max_value() + 100 * 1.9
		assert_err!(
			sum_with_mult_result(Balance::MAX, 100, Rate::saturating_from_rational(19, 10)),
			DispatchError::Other("Overflow Error")
		);
	}
}
