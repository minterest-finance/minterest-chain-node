//! RPC interface for the mnt-token pallet.
//!
//! RPC installation: `rpc/src/lib.rc`
//!
//! Corresponding runtime API declaration: `pallets/mnt-token/rpc/run-time/src/lib.rs`
//! Corresponding runtime API implementation: `runtime/src/lib.rs`

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use minterest_primitives::{OriginalAsset, Rate};
pub use mnt_token_rpc_runtime_api::{MntBalanceInfo, MntTokenRuntimeApi};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
/// Base trait for RPC interface of mnt-token
pub trait MntTokenRpcApi<BlockHash, AccountId> {
	/// Gets MNT accrued but not yet transferred to user
	///
	/// Parameters:
	///  - `&self` :  Self reference
	///  - `account_id`: user account id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`amount`](`MntBalanceInfo::amount`): the MNT accrued but not yet transferred to each
	/// user.
	#[doc(alias = "MNT RPC")]
	#[doc(alias = "MNT mnt_token")]
	#[rpc(name = "mntToken_getUserTotalUnclaimedMntBalance")]
	fn get_user_total_unclaimed_mnt_balance(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> Result<Option<MntBalanceInfo>>;

	/// Return MNT Borrow Rate and MNT Supply Rate values per block for current pool.
	///
	/// Parameters:
	///  - `&self` :  Self reference
	///  - `pool_id`: current pool id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// (borrow_rate, supply_rate): MNT Borrow Rate and MNT Supply Rate values
	///
	/// - [`borrow_rate`](`Rate`): MNT Borrow Rate value
	/// - [`supply_rate`](`Rate`): MNT Supply Rate value
	#[doc(alias = "MNT RPC")]
	#[doc(alias = "MNT mnt_token")]
	#[rpc(name = "mntToken_getPoolMntBorrowAndSupplyRates")]
	fn get_pool_mnt_borrow_and_supply_rates(
		&self,
		pool_id: OriginalAsset,
		at: Option<BlockHash>,
	) -> Result<Option<(Rate, Rate)>>;
}

/// A struct that implements the `MntTokenRpcApi`.
pub struct MntTokenRpcImpl<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> MntTokenRpcImpl<C, B> {
	/// Create new `MntTokenRpcImpl` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

pub enum Error {
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

/// Implementation of 'MntTokenRpcApi'
impl<C, Block, AccountId> MntTokenRpcApi<<Block as BlockT>::Hash, AccountId> for MntTokenRpcImpl<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: MntTokenRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn get_user_total_unclaimed_mnt_balance(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<MntBalanceInfo>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.get_user_total_unclaimed_mnt_balance(&at, account_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get user unclaimed MNT balance.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn get_pool_mnt_borrow_and_supply_rates(
		&self,
		pool_id: OriginalAsset,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<(Rate, Rate)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.get_pool_mnt_borrow_and_supply_rates(&at, pool_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get total borrow and/or supply MNT APY.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}
}
