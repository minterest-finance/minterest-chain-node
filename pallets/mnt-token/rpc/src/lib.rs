//! RPC interface for the mnt-token pallet.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use mnt_token_rpc_runtime_api::{MntBalanceInfo, MntTokenApi as MntTokenRuntimeApi};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait MntTokenApi<BlockHash, AccountId> {
	#[rpc(name = "mntToken_getUnclaimedMntBalance")]
	fn get_unclaimed_mnt_balance(&self, account_id: AccountId, at: Option<BlockHash>)
		-> Result<Option<MntBalanceInfo>>;
}

/// A struct that implements the [`MntTokenApi`].
pub struct MntToken<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> MntToken<C, B> {
	/// Create new `MntToken` with the given reference to the client.
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

impl<C, Block, AccountId> MntTokenApi<<Block as BlockT>::Hash, AccountId> for MntToken<C, Block>
where
	Block: BlockT + sp_runtime::traits::Block + sp_runtime::traits::Block,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: MntTokenRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn get_unclaimed_mnt_balance(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<MntBalanceInfo>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.get_unclaimed_mnt_balance(&at, account_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get user unclaimed MNT balance.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
