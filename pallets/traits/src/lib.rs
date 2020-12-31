#![cfg_attr(not(feature = "std"), no_std)]

use minterest_primitives::{Balance, CurrencyId};
use sp_runtime::DispatchResult;

/// An abstraction of basic borrowing functions
pub trait Borrowing<AccountId> {
	/// Updates the state of the core as a consequence of a borrow action.
	fn update_state_on_borrow(
		who: &AccountId,
		underlying_asset_id: CurrencyId,
		amount_borrowed: Balance,
		account_borrows: Balance,
	) -> DispatchResult;

	/// updates the state of the core as a consequence of a repay action.
	fn update_state_on_repay(
		who: &AccountId,
		underlying_asset_id: CurrencyId,
		repay_amount: Balance,
		account_borrows: Balance,
	) -> DispatchResult;
}
