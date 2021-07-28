use super::*;
use scirust::api::*;
use sp_runtime::traits::CheckedDiv;
use sp_runtime::traits::CheckedSub;
use sp_std::collections::btree_map::BTreeMap;
use std::ops::Neg;

/// Types of liquidation of user loans.
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, PartialOrd, Ord)]
pub enum LiquidationMode {
	/// Makes the user's loan solvent. A portion of the user's borrow is paid from the
	/// liquidation pools, and a portion of the user's collateral is withdrawn and transferred to
	/// the liquidation pools.
	Partial,
	/// All user borrow is paid from liquidation pools. The user's collateral required to cover
	/// the borrow is withdrawn and transferred to liquidation pools.
	Complete,
	/// Occurs when the user's borrow exceeds his supply. This type refers to complete liquidation.
	ForgivableComplete,
}

/// Contains information about the transferred amounts for liquidation.
#[derive(Default, Encode, Decode, RuntimeDebug, Clone, PartialOrd, PartialEq)]
pub struct LiquidationAmounts {
	/// Contains a vector of pools and a balances that must be paid instead of the borrower from
	/// liquidation pools to liquidity pools.
	pub borrower_loans_to_repay_underlying: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balances that must be withdrawn from the user's collateral
	/// and sent to the liquidation pools.
	pub borrower_supplies_to_seize_underlying: Vec<(CurrencyId, Balance)>,
}

#[derive(Default, Clone, Copy)]
pub struct IntermediaryLiquidationValues {
	pub borrowed: Balance,
	pub lended: Balance,
	pub r_val: Rate,
	pub diff: Rate,
	pub r_val_1: Rate,
	pub to_remove_from_borrowed: Balance,
	pub lended_coef: Rate,
	pub collateral_coef: Rate,
	pub lended_x_collateral_coef: Rate,
	pub final_coef: Rate,
}

/// Contains information about the current state of the borrower's loan.
#[derive(Encode, Decode, Eq, PartialEq, Clone, Debug, PartialOrd, Ord)]
pub struct UserLoanState<T>
where
	T: Config,
{
	/// User AccountId whose loan is being processed.
	user: T::AccountId,
	/// Vector of user borrows. Contains information about the CurrencyId and the amount of borrow.
	borrows: Vec<(CurrencyId, Balance)>,
	/// Vector of user supplies. Contains information about the CurrencyId and the amount of supply.
	/// Considers supply only for those pools that are enabled as collateral.
	supplies: Vec<(CurrencyId, Balance)>,
}

// Pub API.
impl<T: Config> UserLoanState<T> {
	/// Constructor.
	pub fn new(user: &T::AccountId) -> Self {
		Self {
			user: user.clone(),
			borrows: Vec::new(),
			supplies: Vec::new(),
		}
	}

	/// Constructs the user's state of loan based on the current state of the storage on
	/// the blockchain. This function internally calls functions from the controller pallet,
	/// which internally call `accrue_interest_rate`.
	///
	/// -`who`: user AccountId whose loan is being processed.
	///
	/// Returns: information about the current state of the borrower's loan.
	pub fn build_user_loan_state(who: &T::AccountId) -> Result<Self, DispatchError> {
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&pool_id| T::LiquidityPoolsManager::pool_exists(&pool_id))
			.try_fold(
				Self::new(who),
				|mut user_loan_state, pool_id| -> Result<Self, DispatchError> {
					let oracle_price =
						T::PriceSource::get_underlying_price(pool_id).ok_or(Error::<T>::InvalidFeedPrice)?;

					let user_borrow_underlying =
						T::ControllerManager::get_user_borrow_underlying_balance(who, pool_id)?;
					if !user_borrow_underlying.is_zero() {
						let user_borrow_usd =
							T::LiquidityPoolsManager::underlying_to_usd(user_borrow_underlying, oracle_price)?;
						user_loan_state.borrows.push((pool_id, user_borrow_usd));
					}

					if T::LiquidityPoolsManager::is_pool_collateral(&who, pool_id) {
						let user_supply_underlying =
							T::ControllerManager::get_user_supply_underlying_balance(who, pool_id)?;
						let user_supply_usd =
							T::LiquidityPoolsManager::underlying_to_usd(user_supply_underlying, oracle_price)?;
						user_loan_state.supplies.push((pool_id, user_supply_usd));
					}
					Ok(user_loan_state)
				},
			)
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

	/// Selects the liquidation mode for the user's loan. The choice of the liquidation mode is
	/// made based on the parameters of the current number of user's liquidation attempts and
	/// the current state of the user's loan.
	///
	/// -`borrower`: user for which the liquidation mode is chosen.
	/// -`user_loan_state`: contains the current state of the borrower's loan.
	///
	/// Returns the `borrower` loan liquidation mode.
	pub fn choose_liquidation_mode(&self) -> Result<LiquidationMode, DispatchError> {
		let (user_total_borrow_usd, user_total_collateral_usd) = (self.total_borrow()?, self.total_collateral()?);
		ensure!(
			user_total_borrow_usd > user_total_collateral_usd,
			Error::<T>::SolventUserLoan
		);
		let user_liquidation_attempts = Pallet::<T>::get_user_liquidation_attempts(&self.user);
		let (user_total_seize_usd, user_total_supply_usd) = (self.total_seize()?, self.total_supply()?);
		if user_total_seize_usd > user_total_supply_usd {
			Ok(LiquidationMode::ForgivableComplete)
		} else if user_total_borrow_usd >= T::PartialLiquidationMinSum::get()
			&& user_liquidation_attempts < T::PartialLiquidationMaxAttempts::get()
		{
			Ok(LiquidationMode::Partial)
		} else {
			Ok(LiquidationMode::Complete)
		}
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for partial liquidation.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	pub fn calculate_partial_liquidation(&self) -> Result<LiquidationAmounts, DispatchError> {
	    let liquidation_threshold = Pallet::<T>::liquidation_threshold_storage();

	    let total_lended = self.total_supply()?;
	    let total_borrowed = self.total_borrow()?;
	    let total_collateral = self.total_collateral()?;
	    let total_collateral_factor = Rate::from_inner(total_collateral).checked_div(&Rate::from_inner(total_lended)).ok_or(Error::<T>::NumOverflow)?;
	    let minimal_supply_ratio = Rate::saturating_from_integer(1).checked_div(&total_collateral_factor).ok_or(Error::<T>::NumOverflow)?;
	    let save_supply_ratio = minimal_supply_ratio.checked_add(&liquidation_threshold).ok_or(Error::<T>::NumOverflow)?;
	    let current_supply_ratio = Rate::from_inner(total_lended).checked_div(&Rate::from_inner(total_borrowed)).ok_or(Error::<T>::NumOverflow)?;

		let mut pool_values = BTreeMap::new();
		/// Copy self.supplies to a map of pool values
		CurrencyId::get_enabled_tokens_in_protocol(UnderlyingAsset)
			.into_iter()
			.filter(|&pool_id| T::LiquidityPoolsManager::pool_exists(&pool_id))
			.for_each(|pool_id| {
				let mut tmp = IntermediaryLiquidationValues::default();
				let supply = self.supplies.iter().find(|(p, _)| *p == pool_id).map(|(_, supply)| *supply).unwrap_or(Balance::zero());
				tmp.lended = supply;
				pool_values.insert(pool_id, tmp);
			});

		/// Fill borrowed and calculate all stuff required for matrix calculations
	    let mut sum_r_vars = Rate::zero();
	    self.borrows.iter().try_for_each(|(pool_id, user_borrow_usd)| -> Result<(), DispatchError> {
    	    let some_var = Rate::from_inner(*user_borrow_usd).checked_div(&Rate::from_inner(total_lended)).ok_or(Error::<T>::NumOverflow)?;
    	    let tmp = pool_values.get_mut(pool_id).ok_or(Error::<T>::NumOverflow)?;
			tmp.borrowed = *user_borrow_usd;
			tmp.r_val = some_var;
			sum_r_vars = sum_r_vars.checked_add(&some_var).ok_or(Error::<T>::NumOverflow)?;
    	    Ok(())
	    })?;
		let x_coef = Rate::saturating_from_integer(1).checked_div(&save_supply_ratio).ok_or(Error::<T>::NumOverflow)?.to_float()
			- Rate::saturating_from_integer(1).checked_div(&current_supply_ratio).ok_or(Error::<T>::NumOverflow)?.to_float();


		/// Sum of collateral of pools where user has positive supply
		let mut sum_used_collateral = Rate::zero();
		pool_values.iter_mut()
			.filter(|(_, tmp)| tmp.r_val.is_positive())
		 	.try_for_each(|(pool_id, tmp)| -> Result<(), DispatchError> {
		 		let diff = tmp.r_val.checked_div(&sum_r_vars).ok_or(Error::<T>::NumOverflow)?;
		 		(*tmp).diff = diff;

				let r_val_1 = Rate::from_float(x_coef * diff.to_float() + tmp.r_val.to_float());
		 		(*tmp).r_val_1 = r_val_1;
				if !tmp.lended.is_zero() {
					sum_used_collateral = sum_used_collateral.checked_add(&T::ControllerManager::get_pool_collateral_factor(*pool_id)).ok_or(Error::<T>::NumOverflow)?;
				}
		 		Ok(())
		 	})?;
		/// Pools with positive borrow
		let mut pools_to_remove_borrowed_from = Vec::new();
		pool_values.iter()
			.filter(|(_, tmp)| tmp.r_val.is_positive())
			.for_each(|(&pool_id, tmp)| {
				if tmp.r_val_1.is_positive() {
					pools_to_remove_borrowed_from.push(pool_id);
				}
			});

        /// Calculate how much to repay for each pool
		let size = pools_to_remove_borrowed_from.len();
		let mut matrix = MatrixF64::zeros(size, size);
		let mut vector = MatrixF64::zeros(size, 1);
		pools_to_remove_borrowed_from.iter().enumerate().try_for_each(|(i, pool_id)| -> Result<(), DispatchError> {
			let tmp = pool_values.get(pool_id).ok_or(Error::<T>::NumOverflow)?;
			let vec_value = Rate::from_inner(tmp.borrowed)
				.checked_sub(&tmp.r_val_1.checked_mul(&Rate::from_inner(total_lended)).ok_or(Error::<T>::NumOverflow)?).ok_or(Error::<T>::NumOverflow)?;
			vector.set(i, 0, vec_value.to_float());

			pools_to_remove_borrowed_from.iter().enumerate().try_for_each(|(j, &pool_id_inner)| -> Result<(), DispatchError> {
				let liquidation_fee = Pallet::<T>::liquidation_fee_storage(pool_id_inner);
				let matrix_value = -(1f64 + liquidation_fee.to_float()) * tmp.r_val_1.to_float() + (if i == j { 1f64 } else { 0f64 });
				matrix.set(i, j, matrix_value);
				Ok(())
			})?;
			Ok(())
		})?;
		let x = GaussElimination::new(&matrix, &vector).solve().map_err(|_| Error::<T>::NumOverflow)?;
		pools_to_remove_borrowed_from.iter().enumerate().try_for_each(|(i, pool_id)| -> Result<(), DispatchError> {
			let tmp = pool_values.get_mut(pool_id).ok_or(Error::<T>::NumOverflow)?;
			(*tmp).to_remove_from_borrowed = Rate::from_float(x.get(i, 0).ok_or(Error::<T>::NumOverflow)?).into_inner();
			Ok(())
		})?;

		///Calculate how much to seize from supply pools
		let mut sum_lended_x_collateral_coef = Rate::zero();
		pool_values.iter_mut().try_for_each(|(pool_id, tmp)| -> Result<(), DispatchError> {
			tmp.lended_coef = Rate::from_inner(tmp.lended).checked_div(&Rate::from_inner(total_lended)).ok_or(Error::<T>::NumOverflow)?;
			tmp.collateral_coef = match tmp.lended.is_zero() {
				true =>	Rate::zero(),
				false => T::ControllerManager::get_pool_collateral_factor(*pool_id).checked_div(&sum_used_collateral).ok_or(Error::<T>::NumOverflow)?,
			};
			tmp.lended_x_collateral_coef = tmp.lended_coef.checked_mul(&tmp.collateral_coef).ok_or(Error::<T>::NumOverflow)?;
			sum_lended_x_collateral_coef = sum_lended_x_collateral_coef.checked_add(&tmp.lended_x_collateral_coef).ok_or(Error::<T>::NumOverflow)?;
			Ok(())
		})?;

		pool_values.iter_mut().try_for_each(|(pool_id, tmp)| -> Result<(), DispatchError> {
			tmp.final_coef = tmp.lended_x_collateral_coef.checked_div(&sum_lended_x_collateral_coef).ok_or(Error::<T>::NumOverflow)?;
			Ok(())
		})?;

		let mut lended_to_seize = BTreeMap::new();
		let mut result = LiquidationAmounts::default();
		pool_values.iter().try_for_each(|(pool_id, tmp)| -> Result<(), DispatchError> {
			result.borrower_loans_to_repay_underlying.push((*pool_id, tmp.to_remove_from_borrowed));
			let liquidation_fee = Pallet::<T>::liquidation_fee_storage(pool_id);
			let to_remove_from_borrowed = tmp.to_remove_from_borrowed;
			pool_values.iter().try_for_each(|(pool_id_inner, tmp)| -> Result<(), DispatchError> {
				let to_seize = Rate::one().checked_add(&liquidation_fee)
					.and_then(|v| v.checked_mul(&Rate::from_inner(to_remove_from_borrowed)))
					.and_then(|v| v.checked_mul(&tmp.final_coef))
					.ok_or(Error::<T>::NumOverflow)?;
				let pool_seize = lended_to_seize.entry(*pool_id_inner).or_insert(Rate::zero());
				*pool_seize = pool_seize.checked_add(&to_seize).ok_or(Error::<T>::NumOverflow)?;
				Ok(())
			})?;
			Ok(())
		})?;

		let mut sum_positive_lended_after_seize = Rate::zero();
		let mut sum_negative_lended_after_seize = Rate::zero();
		pool_values.iter().try_for_each(|(pool_id, tmp)| -> Result<(), DispatchError> {
			let to_seize = *lended_to_seize.get(pool_id).unwrap_or(&Rate::zero());
			let lended_as_rate = Rate::from_inner(tmp.lended);
			if lended_as_rate > to_seize {
				sum_positive_lended_after_seize = lended_as_rate.checked_sub(&to_seize)
					.and_then(|v| v.checked_add(&sum_positive_lended_after_seize)).ok_or(Error::<T>::NumOverflow)?;
			}
			else {
				sum_negative_lended_after_seize = to_seize.checked_sub(&lended_as_rate)
					.and_then(|v| v.checked_add(&sum_negative_lended_after_seize)).ok_or(Error::<T>::NumOverflow)?;
			}
			Ok(())
		})?;

		pool_values.iter().try_for_each(|(pool_id, tmp)| -> Result<(), DispatchError> {
			let pool_seize = lended_to_seize.entry(*pool_id).or_insert(Rate::zero());
			let lended_as_rate = Rate::from_inner(tmp.lended);
			if lended_as_rate > *pool_seize {
				let lended_percent = lended_as_rate.checked_sub(pool_seize)
					.and_then(|v| v.checked_div(&sum_positive_lended_after_seize))
					.ok_or(Error::<T>::NumOverflow)?;
				*pool_seize = lended_percent.checked_mul(&sum_negative_lended_after_seize)
					.and_then(|v| v.checked_add(pool_seize))
					.ok_or(Error::<T>::NumOverflow)?;
			}
			else {
				*pool_seize = Rate::zero();
			}
			result.borrower_supplies_to_seize_underlying.push((*pool_id, pool_seize.into_inner()));
			Ok(())
		})?;

		Ok(result)
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for complete liquidation.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	pub fn calculate_complete_liquidation(&self) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// Based on the current state of the user's insolvent loan, it calculates the amounts required
	/// for "forgivable" complete liquidation. This function is called when user_total_borrow is
	/// greater than user_total_supply.
	///
	/// Returns: vectors with user's borrows to be paid from the liquidation pools instead of
	/// the borrower, and a vector with user's supplies to be withdrawn from the borrower and sent
	/// to the liquidation pools. Balances are calculated in underlying assets.
	///
	/// Note: this function should be used after `accrue_interest_rate`.
	pub fn calculate_forgivable_complete_liquidation(&self) -> Result<LiquidationAmounts, DispatchError> {
		todo!()
	}

	/// Getter for `self.user`.
	pub fn get_user(&self) -> &T::AccountId {
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
}

// private functions
impl<T: Config> UserLoanState<T> {
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
}
