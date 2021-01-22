#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system, ensure_signed};
use minterest_primitives::{CurrencyId, CurrencyPair, Rate};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug,
};
use sp_std::{cmp::Ordering, result, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MinterestModelData {
	/// The utilization point at which the jump multiplier is applied
	pub kink: Rate,

	/// The base interest rate which is the y-intercept when utilization rate is 0
	pub base_rate_per_block: Rate,

	/// The multiplier of utilization rate that gives the slope of the interest rate
	pub multiplier_per_block: Rate,

	/// The multiplierPerBlock after hitting a specified utilization point
	pub jump_multiplier_per_block: Rate,
}

type Accounts<T> = accounts::Module<T>;

pub trait Trait: system::Trait + accounts::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The approximate number of blocks per year
	type BlocksPerYear: Get<u128>;

	/// Enabled currency pairs.
	type EnabledCurrencyPair: Get<Vec<CurrencyPair>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MinterestModel {
		pub MinterestModelDates get(fn minterest_model_dates) config(): map hasher(blake2_128_concat) CurrencyId => MinterestModelData;
	}
}

decl_event!(
	pub enum Event {
		/// JumpMultiplierPerBlock has been successfully changed
		JumpMultiplierPerBlockHasChanged,

		/// BaseRatePerBlock has been successfully changed
		BaseRatePerBlockHasChanged,

		/// MultiplierPerBlock has been successfully changed
		MultiplierPerBlockHasChanged,
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// The currency is not enabled in protocol.
		NotValidUnderlyingAssetId,

		/// Number overflow in calculation.
		NumOverflow,

		/// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
		BaseRatePerBlockCannotBeZero,

		/// Multiplier per block cannot be set to 0 at the same time as Base rate per block.
		MultiplierPerBlockCannotBeZero,

		/// The dispatch origin of this call must be Administrator.
		RequireAdmin,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Set JumpMultiplierPerBlock from JumpMultiplierPerYear.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_jump_multiplier_per_block(origin, pool_id: CurrencyId, jump_multiplier_rate_per_year_n: u128, jump_multiplier_rate_per_year_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<T>::EnabledCurrencyPair::get()
					.iter()
					.any(|pair| pair.underlying_id == pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_jump_multiplier_per_year = Rate::checked_from_rational(jump_multiplier_rate_per_year_n, jump_multiplier_rate_per_year_d).ok_or(Error::<T>::NumOverflow)?;
			let new_jump_multiplier_per_block = new_jump_multiplier_per_year
				.checked_div(&Rate::from_inner(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			MinterestModelDates::mutate(pool_id, |r| r.jump_multiplier_per_block = new_jump_multiplier_per_block);
			Self::deposit_event(Event::JumpMultiplierPerBlockHasChanged);
			Ok(())
		}

		/// Set BaseRatePerBlock from BaseRatePerYear.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_base_rate_per_block(origin, pool_id: CurrencyId, base_rate_per_year_n: u128, base_rate_per_year_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<T>::EnabledCurrencyPair::get()
					.iter()
					.any(|pair| pair.underlying_id == pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_base_rate_per_year = Rate::checked_from_rational(base_rate_per_year_n, base_rate_per_year_d)
				.ok_or(Error::<T>::NumOverflow)?;
			let new_base_rate_per_block = new_base_rate_per_year
				.checked_div(&Rate::from_inner(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Base rate per block cannot be set to 0 at the same time as Multiplier per block.
			if new_base_rate_per_block.is_zero() {
				ensure!(!Self::minterest_model_dates(pool_id).multiplier_per_block.is_zero(), Error::<T>::BaseRatePerBlockCannotBeZero);
			}

			MinterestModelDates::mutate(pool_id, |r| r.base_rate_per_block = new_base_rate_per_block);
			Self::deposit_event(Event::BaseRatePerBlockHasChanged);
			Ok(())
		}

		/// Set MultiplierPerBlock from MultiplierPerYear.
		///
		/// The dispatch origin of this call must be Administrator.
		#[weight = 0]
		pub fn set_multiplier_per_block(origin, pool_id: CurrencyId, multiplier_rate_per_year_n: u128, multiplier_rate_per_year_d: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::is_admin_internal(&sender), Error::<T>::RequireAdmin);

			ensure!(
				<T>::EnabledCurrencyPair::get()
					.iter()
					.any(|pair| pair.underlying_id == pool_id),
				Error::<T>::NotValidUnderlyingAssetId
			);

			let new_multiplier_per_year = Rate::checked_from_rational(multiplier_rate_per_year_n, multiplier_rate_per_year_d)
				.ok_or(Error::<T>::NumOverflow)?;
			let new_multiplier_per_block = new_multiplier_per_year
				.checked_div(&Rate::from_inner(T::BlocksPerYear::get()))
				.ok_or(Error::<T>::NumOverflow)?;

			// Multiplier per block cannot be set to 0 at the same time as Base rate per block .
			if new_multiplier_per_block.is_zero() {
				ensure!(!Self::minterest_model_dates(pool_id).base_rate_per_block.is_zero(), Error::<T>::MultiplierPerBlockCannotBeZero);
			}

			MinterestModelDates::mutate(pool_id, |r| r.multiplier_per_block = new_multiplier_per_block);
			Self::deposit_event(Event::MultiplierPerBlockHasChanged);
			Ok(())
		}
	}
}

type RateResult = result::Result<Rate, DispatchError>;

impl<T: Trait> Module<T> {
	/// Calculates the current borrow rate per block.
	pub fn calculate_borrow_interest_rate(underlying_asset_id: CurrencyId, utilization_rate: Rate) -> RateResult {
		let kink = Self::minterest_model_dates(underlying_asset_id).kink;
		let multiplier_per_block = Self::minterest_model_dates(underlying_asset_id).multiplier_per_block;
		let base_rate_per_block = Self::minterest_model_dates(underlying_asset_id).base_rate_per_block;

		let borrow_interest_rate = match utilization_rate.cmp(&kink) {
			Ordering::Greater => {
				let jump_multiplier_per_block =
					Self::minterest_model_dates(underlying_asset_id).jump_multiplier_per_block;
				let normal_rate = kink
					.checked_mul(&multiplier_per_block)
					.and_then(|v| v.checked_add(&base_rate_per_block))
					.ok_or(Error::<T>::NumOverflow)?;
				let excess_util = utilization_rate.checked_mul(&kink).ok_or(Error::<T>::NumOverflow)?;

				excess_util
					.checked_mul(&jump_multiplier_per_block)
					.and_then(|v| v.checked_add(&normal_rate))
					.ok_or(Error::<T>::NumOverflow)?
			}
			_ => utilization_rate
				.checked_mul(&multiplier_per_block)
				.and_then(|v| v.checked_add(&base_rate_per_block))
				.ok_or(Error::<T>::NumOverflow)?,
		};

		Ok(borrow_interest_rate)
	}
}
