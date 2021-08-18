#![allow(clippy::comparison_chain)]

use super::*;
use nalgebra::{DMatrix, DVector};
use sp_runtime::traits::{CheckedDiv, CheckedSub};
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug};

type LiquidationAmountsResult = Result<(Vec<(CurrencyId, Balance)>, Vec<(CurrencyId, Balance)>), DispatchError>;
type CompleteLiquidationAmountsResult = Result<
	(
		Vec<(CurrencyId, Balance)>,
		Vec<(CurrencyId, Balance)>,
		Vec<(CurrencyId, Balance)>,
	),
	DispatchError,
>;

/// Types of liquidation of user loans.
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, PartialOrd, Ord)]
pub enum LiquidationMode {
	/// Makes the user's loan solvent. A portion of the user's borrow is paid from the
	/// liquidation pools, and a portion of the user's collateral is withdrawn and transferred to
	/// the liquidation pools.
	Partial,
	/// All user borrow is paid from liquidation pools. The user's collateral required to cover
	/// the borrow is withdrawn and transferred to liquidation pools.
	/// In case if user`s borrow exceeds his supply liquidation pools will be used to cover the
	/// difference
	Complete,
}

/// Contains information about the current state of the borrower's loan.
#[derive(Encode, Decode, Eq, PartialEq, Clone, Debug, PartialOrd, Ord)]
pub struct UserLoanState<T>
where
	T: Config + Debug,
{
	/// User AccountId whose loan is being processed.
	user: T::AccountId,
	/// Vector of user borrows. Contains information about the CurrencyId and the amount of borrow.
	borrows: Vec<(CurrencyId, Balance)>,
	/// Vector of user supplies. Contains information about the CurrencyId and the amount of supply.
	/// Considers supply only for those pools that are enabled as collateral.
	supplies: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balances that must be paid instead of the borrower from
	/// liquidation pools to liquidity pools.
	borrows_to_repay_underlying: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balances that must be withdrawn from the user's collateral
	/// and sent to the liquidation pools.
	supplies_to_seize_underlying: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balance that must be paid from the liquidation pools
	/// to liquidity pools.
	supplies_to_pay_underlying: Vec<(CurrencyId, Balance)>,
	/// Type of liquidation of user loans.
	liquidation_mode: Option<LiquidationMode>,
}

// Pub API.
impl<T: Config + Debug> UserLoanState<T> {
	/// Constructs the user's state of loan based on the current state of the storage on
	/// the blockchain.
	///
	/// -`who`: user AccountId whose loan is being processed.
	///
	/// Returns: information about the current state of the borrower's loan.
	pub fn build_user_loan_state(who: &T::AccountId) -> Result<Self, DispatchError> {
		let mut user_loan_state = UserLoanState::new(who);

		let (supplies, borrows) = Self::calculate_user_loan_state(who)?;
		user_loan_state.supplies = supplies;
		user_loan_state.borrows = borrows;

		user_loan_state.liquidation_mode = user_loan_state.choose_liquidation_mode().ok();

		let (supplies_to_seize_underlying, borrows_to_repay_underlying) = match user_loan_state
			.liquidation_mode
			.as_ref()
			.ok_or(Error::<T>::SolventUserLoan)?
		{
			LiquidationMode::Partial => user_loan_state.calculate_partial_liquidation()?,
			LiquidationMode::Complete => {
				let (supplies_to_seize_underlying, borrows_to_repay_underlying, supplies_to_pay_underlying) =
					user_loan_state.calculate_complete_liquidation()?;
				user_loan_state.supplies_to_pay_underlying = supplies_to_pay_underlying;
				(supplies_to_seize_underlying, borrows_to_repay_underlying)
			}
		};

		user_loan_state.supplies_to_seize_underlying = supplies_to_seize_underlying;
		user_loan_state.borrows_to_repay_underlying = borrows_to_repay_underlying;

		Ok(user_loan_state.clone())
	}

	/// Calculates user_total_borrow_usd.
	/// Returns: `user_total_borrow_usd = Sum(user_borrow_usd)`.
	pub fn total_borrow(&self) -> Result<Balance, DispatchError> {
		self.borrows
			.iter()
			.try_fold(Balance::zero(), |acc, (_, borrow_amount)| {
				Ok(acc.checked_add(*borrow_amount).ok_or(Error::<T>::NumOverflow)?)
			})
	}

	/// Calculates user_total_supply.
	/// Returns: `user_total_supply = Sum(user_supply)`.
	pub fn total_supply(&self) -> Result<Balance, DispatchError> {
		self.supplies
			.iter()
			.try_fold(Balance::zero(), |acc, (_, supply_amount)| {
				Ok(acc.checked_add(*supply_amount).ok_or(Error::<T>::NumOverflow)?)
			})
	}

	/// Calculates user_total_seize.
	/// Returns: `user_total_seize = sum(user_borrow * liquidation_fee)`.
	pub fn total_seize(&self) -> Result<Balance, DispatchError> {
		self.borrows.iter().try_fold(
			Balance::zero(),
			|acc, (pool_id, borrow_usd)| -> Result<Balance, DispatchError> {
				let seize_usd = Self::calculate_seize_amount(*pool_id, *borrow_usd)?;
				Ok(acc.checked_add(seize_usd).ok_or(Error::<T>::NumOverflow)?)
			},
		)
	}

	/// Calculates user_total_collateral.
	/// Returns: `user_total_collateral = Sum(user_supply * pool_collateral_factor)`.
	pub fn total_collateral(&self) -> Result<Balance, DispatchError> {
		self.supplies
			.iter()
			.try_fold(Balance::zero(), |acc, (pool_id, supply_amount)| {
				let collateral_amount = T::ControllerManager::calculate_collateral(*pool_id, *supply_amount);
				Ok(acc.checked_add(collateral_amount).ok_or(Error::<T>::NumOverflow)?)
			})
	}

	/// Getter for `self.user`.
	pub fn get_user_account_id(&self) -> &T::AccountId {
		&self.user
	}

	/// Getter for `self.supplies`.
	pub fn get_user_supplies(&self) -> Vec<(CurrencyId, Balance)> {
		self.supplies.clone()
	}

	/// Getter for `self.borrows`.
	pub fn get_user_borrows(&self) -> Vec<(CurrencyId, Balance)> {
		self.borrows.clone()
	}

	/// Getter for `self.liquidation_mode`.
	pub fn get_user_liquidation_mode(&self) -> Option<LiquidationMode> {
		self.liquidation_mode.clone()
	}

	/// Getter for `self.borrows_to_repay_underlying`.
	pub fn get_user_borrows_to_repay_underlying(&self) -> Vec<(CurrencyId, Balance)> {
		self.borrows_to_repay_underlying.clone()
	}

	/// Getter for `self.supplies_to_seize_underlying`.
	pub fn get_user_supplies_to_seize_underlying(&self) -> Vec<(CurrencyId, Balance)> {
		self.supplies_to_seize_underlying.clone()
	}

	/// Getter for `self.supplies_to_pay_underlying`.
	pub fn get_user_supplies_to_pay_underlying(&self) -> Vec<(CurrencyId, Balance)> {
		self.supplies_to_pay_underlying.clone()
	}
}

// private functions
impl<T: Config + Debug> UserLoanState<T> {
	pub(crate) fn new(user: &T::AccountId) -> Self {
		Self {
			user: user.clone(),
			borrows: Vec::new(),
			supplies: Vec::new(),
			borrows_to_repay_underlying: Vec::new(),
			supplies_to_seize_underlying: Vec::new(),
			supplies_to_pay_underlying: Vec::new(),
			liquidation_mode: None,
		}
	}

	/// Calculates the amount to be seized from user's supply (including liquidation fee).
	/// Reads the liquidation fee value from storage.
	///
	/// Returns: `seize_amount = borrow_amount * (1 + liquidation_fee)`.
	pub(crate) fn calculate_seize_amount(
		pool_id: CurrencyId,
		borrow_amount: Balance,
	) -> Result<Balance, DispatchError> {
		let liquidation_fee = Pallet::<T>::liquidation_fee_storage(pool_id);
		let seize_amount = Rate::one()
			.checked_add(&liquidation_fee)
			.and_then(|v| v.checked_mul(&Rate::from_inner(borrow_amount)))
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;
		Ok(seize_amount)
	}

	/// Selects the liquidation mode for the user's loan. The choice of the liquidation mode is
	/// made based on the parameters of the current number of user's liquidation attempts and
	/// the current state of the user's loan.
	///
	/// -`borrower`: user for which the liquidation mode is chosen.
	/// -`user_loan_state`: contains the current state of the borrower's loan.
	///
	/// Returns the `borrower` loan liquidation mode.
	pub(crate) fn choose_liquidation_mode(&self) -> Result<LiquidationMode, DispatchError> {
		let (user_total_borrow_usd, user_total_collateral_usd) = (self.total_borrow()?, self.total_collateral()?);
		ensure!(
			user_total_borrow_usd > user_total_collateral_usd,
			Error::<T>::SolventUserLoan
		);
		let user_liquidation_attempts = Pallet::<T>::get_user_liquidation_attempts(&self.user);
		let (user_total_seize_usd, user_total_supply_usd) = (self.total_seize()?, self.total_supply()?);
		if user_total_seize_usd > user_total_supply_usd
			|| user_total_borrow_usd < T::PartialLiquidationMinSum::get()
			|| user_liquidation_attempts >= T::PartialLiquidationMaxAttempts::get()
		{
			Ok(LiquidationMode::Complete)
		} else {
			Ok(LiquidationMode::Partial)
		}
	}

	/// Calculates user supply and borrows across all liquidity pools. Considers supply only in
	/// liquidity pools that are enabled as collateral. This function internally calls functions
	/// from the controller pallet, which internally call `accrue_interest_rate`.
	///
	/// -`who`: user AccountId whose loan is being processed.
	///
	/// Returns: information about the current state of the borrower's loan.
	pub(crate) fn calculate_user_loan_state(who: &T::AccountId) -> LiquidationAmountsResult {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&pool_id| T::LiquidityPoolsManager::pool_exists(&pool_id))
			.try_fold(
				(Vec::new(), Vec::new()),
				|(mut supplies, mut borrows), pool_id| -> LiquidationAmountsResult {
					let oracle_price =
						T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

					let user_borrow_underlying =
						T::ControllerManager::get_user_borrow_underlying_balance(who, pool_id)?;
					if !user_borrow_underlying.is_zero() {
						let user_borrow_usd =
							T::LiquidityPoolsManager::underlying_to_usd(user_borrow_underlying, oracle_price)?;
						borrows.push((pool_id, user_borrow_usd));
					}

					if T::LiquidityPoolsManager::is_pool_collateral(&who, pool_id) {
						let user_supply_underlying =
							T::ControllerManager::get_user_supply_underlying_balance(who, pool_id)?;
						let user_supply_usd =
							T::LiquidityPoolsManager::underlying_to_usd(user_supply_underlying, oracle_price)?;
						supplies.push((pool_id, user_supply_usd));
					}
					Ok((supplies, borrows))
				},
			)
	}

	/// For each pool calculates the amount to reduce user`s borrow for
	///
	/// Returns: vector of (pool_id, repay_amount) for each pool where repay_amount is > 0
	pub(crate) fn calculate_borrower_loans_to_repay(&self) -> Result<Vec<(CurrencyId, Balance)>, DispatchError> {
		#[derive(Default, Clone, Copy)]
		struct PoolUserIntermediaryLiquidationValues {
			/// User borrow for a pool
			pub borrow_usd: Balance,
			/// User supply for a pool
			pub supply_usd: Balance,
			/// User borrow for a pool divided by sum(supply)
			pub borrowed_to_total_supply_ratio: Rate,
			/// borrowed_to_total_supply_ratio which we need to achieve
			pub borrowed_to_total_supply_ratio_new: Rate,
		}

		let liquidation_threshold = Pallet::<T>::liquidation_threshold_storage();
		let total_supply = self.total_supply()?;
		let total_borrowed = self.total_borrow()?;
		let total_collateral = self.total_collateral()?;
		let total_collateral_factor = Rate::from_inner(total_collateral)
			.checked_div(&Rate::from_inner(total_supply))
			.ok_or(Error::<T>::NumOverflow)?;
		let minimal_supply_ratio = Rate::one()
			.checked_div(&total_collateral_factor)
			.ok_or(Error::<T>::NumOverflow)?;
		let save_supply_ratio = minimal_supply_ratio
			.checked_add(&liquidation_threshold)
			.ok_or(Error::<T>::NumOverflow)?;
		let current_supply_ratio = Rate::from_inner(total_supply)
			.checked_div(&Rate::from_inner(total_borrowed))
			.ok_or(Error::<T>::NumOverflow)?;

		let mut pool_to_liquidation_values = BTreeMap::new();
		// Copy self.supplies to a map of pool values
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&pool_id| T::LiquidityPoolsManager::pool_exists(&pool_id))
			.for_each(|pool_id| {
				let mut liquidation_values = PoolUserIntermediaryLiquidationValues::default();
				let supply = self
					.supplies
					.iter()
					.find(|(p, _)| *p == pool_id)
					.map(|(_, supply)| *supply)
					.unwrap_or_else(Balance::zero);
				liquidation_values.supply_usd = supply;
				pool_to_liquidation_values.insert(pool_id, liquidation_values);
			});

		// Fill borrow_usd and calculate all stuff required for matrix calculations
		let mut sum_borrowed_to_total_supply_ratio = Rate::zero();
		self.borrows
			.iter()
			.try_for_each(|(pool_id, user_borrow_usd)| -> Result<(), DispatchError> {
				let borrowed_to_total_supply_ratio = Rate::from_inner(*user_borrow_usd)
					.checked_div(&Rate::from_inner(total_supply))
					.ok_or(Error::<T>::NumOverflow)?;
				let liquidation_values = pool_to_liquidation_values
					.get_mut(pool_id)
					.ok_or(Error::<T>::LiquidationMathFailed)?; // should not happen
				liquidation_values.borrow_usd = *user_borrow_usd;
				liquidation_values.borrowed_to_total_supply_ratio = borrowed_to_total_supply_ratio;
				sum_borrowed_to_total_supply_ratio = sum_borrowed_to_total_supply_ratio
					.checked_add(&borrowed_to_total_supply_ratio)
					.ok_or(Error::<T>::NumOverflow)?;
				Ok(())
			})?;

		/// TODO: 	type `Rate` is not supported for `no_std` by `to_float()`, `from_float` methods
		let x_coef = to_float(
			Rate::one()
				.checked_div(&save_supply_ratio)
				.ok_or(Error::<T>::NumOverflow)?,
		) - to_float(
			Rate::one()
				.checked_div(&current_supply_ratio)
				.ok_or(Error::<T>::NumOverflow)?,
		);

		// Sum of collateral of pools where user has positive supply
		let mut sum_used_collateral = Rate::zero();
		pool_to_liquidation_values
			.iter_mut()
			.filter(|(_, liquidation_values)| liquidation_values.borrowed_to_total_supply_ratio.is_positive())
			.try_for_each(|(pool_id, liquidation_values)| -> Result<(), DispatchError> {
				let borrowed_to_total_supply_ratio_percentage = liquidation_values
					.borrowed_to_total_supply_ratio
					.checked_div(&sum_borrowed_to_total_supply_ratio)
					.ok_or(Error::<T>::NumOverflow)?;

				let borrowed_to_total_supply_ratio_new = Rate::from_inner(
					(x_coef * to_float(borrowed_to_total_supply_ratio_percentage)
						+ to_float(liquidation_values.borrowed_to_total_supply_ratio)) as u128,
				);

				liquidation_values.borrowed_to_total_supply_ratio_new = borrowed_to_total_supply_ratio_new;
				if !liquidation_values.supply_usd.is_zero() {
					sum_used_collateral = sum_used_collateral
						.checked_add(&T::ControllerManager::get_pool_collateral_factor(*pool_id))
						.ok_or(Error::<T>::NumOverflow)?;
				}
				Ok(())
			})?;
		// Pools with positive borrow
		let mut pools_to_remove_borrowed_from = Vec::new();
		pool_to_liquidation_values
			.iter()
			.filter(|(_, liquidation_values)| liquidation_values.borrowed_to_total_supply_ratio.is_positive())
			.for_each(|(&pool_id, liquidation_values)| {
				if liquidation_values.borrowed_to_total_supply_ratio_new.is_positive() {
					pools_to_remove_borrowed_from.push(pool_id);
				}
			});

		// Calculate how much to repay for each pool
		let size = pools_to_remove_borrowed_from.len();

		let mut matrix = DMatrix::zeros(size, size);
		let mut vector = DVector::zeros(size);
		pools_to_remove_borrowed_from
			.iter()
			.enumerate()
			.try_for_each(|(i, pool_id)| -> Result<(), DispatchError> {
				let liquidation_values = pool_to_liquidation_values
					.get(pool_id)
					.ok_or(Error::<T>::LiquidationMathFailed)?; // should not happen
				let vec_value = Rate::from_inner(liquidation_values.borrow_usd)
					.checked_sub(
						&liquidation_values
							.borrowed_to_total_supply_ratio_new
							.checked_mul(&Rate::from_inner(total_supply))
							.ok_or(Error::<T>::NumOverflow)?,
					)
					.ok_or(Error::<T>::NumOverflow)?;

				vector[(i, 0)] = to_float(vec_value);

				pools_to_remove_borrowed_from.iter().enumerate().try_for_each(
					|(j, &pool_id_inner)| -> Result<(), DispatchError> {
						let liquidation_fee = Pallet::<T>::liquidation_fee_storage(pool_id_inner);
						let matrix_value = -(1f64 + to_float(liquidation_fee))
							* to_float(liquidation_values.borrowed_to_total_supply_ratio_new)
							+ (if i == j { 1f64 } else { 0f64 });
						matrix[(i, j)] = matrix_value;
						Ok(())
					},
				)?;
				Ok(())
			})?;
		let x = matrix.lu().solve(&vector).ok_or(Error::<T>::LiquidationMathFailed)?;

		let mut borrower_loans_to_repay = Vec::new();
		pools_to_remove_borrowed_from
			.iter()
			.enumerate()
			.try_for_each(|(i, pool_id)| -> Result<(), DispatchError> {
				borrower_loans_to_repay.push((
					*pool_id,
					Rate::from_inner(*x.get(i).ok_or(Error::<T>::LiquidationMathFailed)? as u128).into_inner(),
				));
				Ok(())
			})?;
		Ok(borrower_loans_to_repay)
	}

	/// For each pool calculates the amount to reduce user`s supply for
	///
	/// Returns: vector of (pool_id, seize_amount) for each pool where seize_amount is > 0
	pub(crate) fn calculate_borrower_supplies_to_seize(
		&self,
		borrower_loans_to_repay: &[(CurrencyId, Balance)],
	) -> Result<Vec<(CurrencyId, Balance)>, DispatchError> {
		#[derive(Default, Clone, Copy)]
		struct PoolUserIntermediaryLiquidationValues {
			/// User supply for a pool
			pub supply_usd: Balance,
			/// User supply for a pool divided by a sum(supply)
			pub supply_ratio: Rate,
			/// Collateral for a pool divided by a sum(collateral)
			pub collateral_ratio: Rate,
			/// (supply_ratio * collateral_ratio) / sum(supply_ratio * collateral_ratio)
			pub supply_seize_factor: Rate,
		}

		let mut pool_to_liquidation_values = BTreeMap::new();
		// Sum of collateral of pools where user has positive supply
		let mut sum_used_collateral = Rate::zero();
		// Copy self.supplies to a map of pool values
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&pool_id| T::LiquidityPoolsManager::pool_exists(&pool_id))
			.try_for_each(|pool_id| -> Result<(), DispatchError> {
				let mut liquidation_values = PoolUserIntermediaryLiquidationValues::default();
				let supply = self
					.supplies
					.iter()
					.find(|(p, _)| *p == pool_id)
					.map(|(_, supply)| *supply)
					.unwrap_or_else(Balance::zero);
				if !supply.is_zero() {
					sum_used_collateral = sum_used_collateral
						.checked_add(&T::ControllerManager::get_pool_collateral_factor(pool_id))
						.ok_or(Error::<T>::NumOverflow)?;
				}
				liquidation_values.supply_usd = supply;
				pool_to_liquidation_values.insert(pool_id, liquidation_values);
				Ok(())
			})?;

		// Calculate how much to seize from supply pools
		let mut sum_of_supply_and_collateral_ratio_product = Rate::zero();
		pool_to_liquidation_values.iter_mut().try_for_each(
			|(pool_id, liquidation_values)| -> Result<(), DispatchError> {
				liquidation_values.supply_ratio = Rate::from_inner(liquidation_values.supply_usd)
					.checked_div(&Rate::from_inner(self.total_supply()?))
					.ok_or(Error::<T>::NumOverflow)?;
				liquidation_values.collateral_ratio = match liquidation_values.supply_usd.is_zero() {
					true => Rate::zero(),
					false => T::ControllerManager::get_pool_collateral_factor(*pool_id)
						.checked_div(&sum_used_collateral)
						.ok_or(Error::<T>::NumOverflow)?,
				};
				let supply_and_collateral_ratio_product = liquidation_values
					.supply_ratio
					.checked_mul(&liquidation_values.collateral_ratio)
					.ok_or(Error::<T>::NumOverflow)?;
				sum_of_supply_and_collateral_ratio_product = sum_of_supply_and_collateral_ratio_product
					.checked_add(&supply_and_collateral_ratio_product)
					.ok_or(Error::<T>::NumOverflow)?;
				Ok(())
			},
		)?;

		pool_to_liquidation_values
			.iter_mut()
			.try_for_each(|(_, liquidation_values)| -> Result<(), DispatchError> {
				liquidation_values.supply_seize_factor = liquidation_values
					.supply_ratio
					.checked_mul(&liquidation_values.collateral_ratio)
					.and_then(|v| v.checked_div(&sum_of_supply_and_collateral_ratio_product))
					.ok_or(Error::<T>::NumOverflow)?;
				Ok(())
			})?;

		let mut supply_to_seize = BTreeMap::new();
		borrower_loans_to_repay.iter().try_for_each(
			|(pool_id, borrower_loan_to_repay)| -> Result<(), DispatchError> {
				let liquidation_fee = Pallet::<T>::liquidation_fee_storage(pool_id);
				pool_to_liquidation_values.iter().try_for_each(
					|(pool_id_inner, liquidation_values)| -> Result<(), DispatchError> {
						let to_seize = Rate::one()
							.checked_add(&liquidation_fee)
							.and_then(|v| v.checked_mul(&Rate::from_inner(*borrower_loan_to_repay)))
							.and_then(|v| v.checked_mul(&liquidation_values.supply_seize_factor))
							.ok_or(Error::<T>::NumOverflow)?;
						let pool_seize = supply_to_seize.entry(*pool_id_inner).or_insert_with(Rate::zero);
						*pool_seize = pool_seize.checked_add(&to_seize).ok_or(Error::<T>::NumOverflow)?;
						Ok(())
					},
				)?;
				Ok(())
			},
		)?;

		let mut sum_positive_supply_after_seize = Rate::zero();
		let mut sum_negative_supply_after_seize = Rate::zero();
		pool_to_liquidation_values.iter().try_for_each(
			|(pool_id, liquidation_values)| -> Result<(), DispatchError> {
				let to_seize = *supply_to_seize.get(pool_id).unwrap_or(&Rate::zero());
				let supply_as_rate = Rate::from_inner(liquidation_values.supply_usd);
				if supply_as_rate > to_seize {
					sum_positive_supply_after_seize = supply_as_rate
						.checked_sub(&to_seize)
						.and_then(|v| v.checked_add(&sum_positive_supply_after_seize))
						.ok_or(Error::<T>::NumOverflow)?;
				} else {
					sum_negative_supply_after_seize = to_seize
						.checked_sub(&supply_as_rate)
						.and_then(|v| v.checked_add(&sum_negative_supply_after_seize))
						.ok_or(Error::<T>::NumOverflow)?;
				}
				Ok(())
			},
		)?;

		// At this point some of entries in supply_to_seize may be greater than supply amounts for
		// respective pools. This means pool doesn't have enough supply to repay borrow and we need to
		// split this shortage between pools that have extra supply
		let mut borrower_supply_to_seize = Vec::new();
		pool_to_liquidation_values.iter().try_for_each(
			|(pool_id, liquidation_values)| -> Result<(), DispatchError> {
				let pool_seize = supply_to_seize.entry(*pool_id).or_insert_with(Rate::zero);
				let supply_as_rate = Rate::from_inner(liquidation_values.supply_usd);
				if sum_positive_supply_after_seize > sum_negative_supply_after_seize {
					// Pool has extra supply -> increase pool_seize by a portion of sum_negative_supply_after_seize
					if supply_as_rate > *pool_seize {
						let supply_percent = supply_as_rate
							.checked_sub(pool_seize)
							.and_then(|v| v.checked_div(&sum_positive_supply_after_seize))
							.ok_or(Error::<T>::NumOverflow)?;
						*pool_seize = supply_percent
							.checked_mul(&sum_negative_supply_after_seize)
							.and_then(|v| v.checked_add(pool_seize))
							.ok_or(Error::<T>::NumOverflow)?;
					} else {
						// Pool has supply shortage which is handled in the above if branch
						*pool_seize = supply_as_rate;
					}
				} else if sum_positive_supply_after_seize < sum_negative_supply_after_seize {
					// Pool has shortage -> decrease pool_seize by a portion of sum_positive_supply_after_seize
					if supply_as_rate < *pool_seize {
						let supply_percent = pool_seize
							.checked_sub(&supply_as_rate)
							.and_then(|v| v.checked_div(&sum_negative_supply_after_seize))
							.ok_or(Error::<T>::NumOverflow)?;
						*pool_seize = pool_seize
							.checked_sub(
								&supply_percent
									.checked_mul(&sum_positive_supply_after_seize)
									.ok_or(Error::<T>::NumOverflow)?,
							)
							.ok_or(Error::<T>::NumOverflow)?;
					} else {
						// Pool has extra supply which is handled in the above if branch
						*pool_seize = supply_as_rate;
					}
				}
				// Total extra supply is equal to total shortage -> just seize all supply
				else {
					*pool_seize = supply_as_rate;
				}
				if !pool_seize.is_zero() {
					borrower_supply_to_seize.push((*pool_id, pool_seize.into_inner()));
				}
				Ok(())
			},
		)?;
		Ok(borrower_supply_to_seize)
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for partial liquidation.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	pub(crate) fn calculate_partial_liquidation(&self) -> LiquidationAmountsResult {
		let borrower_loans_to_repay_underlying = self.calculate_borrower_loans_to_repay()?;
		let borrower_supplies_to_seize_underlying =
			self.calculate_borrower_supplies_to_seize(&borrower_loans_to_repay_underlying)?;
		Ok((
			borrower_supplies_to_seize_underlying,
			borrower_loans_to_repay_underlying,
		))
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for complete liquidation.
	///
	/// Returns:
	/// - vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower;
	/// - vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools;
	/// - vector of pools and a balance that must be paid from the liquidation pools
	/// to liquidity pools. This vector is not empty only if user_seize_ > user_supply.
	/// Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	pub(crate) fn calculate_complete_liquidation(&self) -> CompleteLiquidationAmountsResult {
		let borrower_loans_to_repay_underlying = self.borrows.to_vec();
		let mut borrower_supplies_to_seize_underlying =
			self.calculate_borrower_supplies_to_seize(&borrower_loans_to_repay_underlying)?;
		let mut borrower_supplies_to_pay_underlying = Vec::new();
		borrower_supplies_to_seize_underlying.iter_mut().try_for_each(
			|(pool_id, to_seize)| -> Result<(), DispatchError> {
				let supply = self
					.supplies
					.iter()
					.find(|(p, _)| *p == *pool_id)
					.map(|(_, supply)| *supply)
					.unwrap_or_else(Balance::zero);
				if *to_seize > supply {
					let to_pay = to_seize.checked_sub(supply).ok_or(Error::<T>::NumOverflow)?;
					borrower_supplies_to_pay_underlying.push((*pool_id, to_pay));
					*to_seize = supply;
				}
				Ok(())
			},
		)?;

		Ok((
			borrower_supplies_to_seize_underlying,
			borrower_loans_to_repay_underlying,
			borrower_supplies_to_pay_underlying,
		))
	}
}

/// TODO: temporary replacement of the implementation of the `to_float()` method for the macro
/// `implement_fixed!`
pub(crate) fn to_float(rate: Rate) -> f64 {
	rate.into_inner() as f64 / 1_000_000_000_000_000_000_u128 as f64
}
