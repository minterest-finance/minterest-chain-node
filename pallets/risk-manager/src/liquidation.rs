use super::*;

/// Types of liquidation of user loans.
enum LiquidationMode {
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
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialOrd, PartialEq)]
pub struct LiquidationAmounts {
	/// Contains a vector of pools and a balances that must be paid instead of the borrower from
	/// liquidation pools to liquidity pools.
	pub borrower_loans_to_repay_underlying: Vec<(CurrencyId, Balance)>,
	/// Contains a vector of pools and a balances that must be withdrawn from the user's collateral
	/// and sent to the liquidation pools.
	pub borrower_supplies_to_seize_underlying: Vec<(CurrencyId, Balance)>,
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
		ensure!(
			self.total_borrow()? > self.total_collateral()?,
			Error::<T>::SolventUserLoan
		);
		let user_liquidation_attempts = Pallet::<T>::get_user_liquidation_attempts(&self.user);

		if self.total_seize()? > self.total_supply()? {
			Ok(LiquidationMode::ForgivableComplete)
		} else if self.total_borrow()? <= T::PartialLiquidationMinSum::get()
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
		todo!()
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
