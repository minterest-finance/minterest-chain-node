//! RPC interface for the controller module.

pub use accounts_rpc_runtime_api::AccountsApi as AccountsRuntimeApi;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait AccountsApi<BlockHash, AccountId> {
	#[rpc(name = "accounts_isAdmin")]
	fn is_admin_rpc(&self, caller: AccountId, at: Option<BlockHash>) -> Result<Option<bool>>;
}

/// A struct that implements the [`AccountsApi`].
pub struct Accounts<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> Accounts<C, B> {
	/// Create new `Accounts` with the given reference to the client.
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

impl<C, Block, AccountId> AccountsApi<<Block as BlockT>::Hash, AccountId> for Accounts<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AccountsRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn is_admin_rpc(&self, caller: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<bool>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.is_admin_rpc(&at, caller).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to check if is an admin.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
