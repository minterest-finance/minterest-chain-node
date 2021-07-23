//! RPC interface for the whitelist module.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
pub use whitelist_rpc_runtime_api::WhitelistRuntimeApi;

#[rpc]
/// Base trait for RPC interface of whitelist module
pub trait WhitelistRpcApi<BlockHash, AccountId> {
	/// Checks whether the user is a member of the whitelist.
	///
	///  - `&self` :  Self reference
	///  - `who`: checked account id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return
	/// - is_admin: true / false
	#[doc(alias("MNT RPC", "MNT whitelist_module"))]
	#[rpc(name = "whitelist_isWhitelistMember")]
	fn is_whitelist_member(&self, who: AccountId, at: Option<BlockHash>) -> Result<bool>;
}

/// A struct that implements the [`WhitelistApi`].
pub struct WhitelistRpcImpl<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> WhitelistRpcImpl<C, B> {
	/// Create new `WhitelistRpcImpl` with the given reference to the client.
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

/// Implementation of 'WhitelistRpcApi'
impl<C, Block, AccountId> WhitelistRpcApi<<Block as BlockT>::Hash, AccountId> for WhitelistRpcImpl<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: WhitelistRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn is_whitelist_member(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<bool> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.is_whitelist_member(&at, who).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to check if it is a whitelist member.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
