//! RPC interface for prices pallet
//!
//! RPC installation: `rpc/src/lib.rc`
//!
//! Corresponding runtime API declaration: `pallets/prices/rpc/run-time/src/lib.rs`
//! Corresponding runtime API implementation: `runtime/src/lib.rs`

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use minterest_primitives::{CurrencyId, Price};
pub use prices_rpc_runtime_api::PricesApi as PricesRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
/// Base trait for RPC interface of prices
pub trait PricesApi<BlockHash> {
	/// This function returns a price for a currency.
	/// If currency price has been locked, locked value will be returned.
	/// Otherwise the value from Oracle RMS will be returned
	///
	///  - `&self` :  Self reference
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	#[rpc(name = "prices_getCurrentPrice")]
	fn get_current_price(&self, at: Option<BlockHash>, currency_id: CurrencyId) -> Result<Option<Price>>;

	/// This function returns a Vector containing prices for all currencies been locked
	/// In case some currency prices were not locked, None will be returned for corresponding
	/// currencies.
	/// Function read prices values from local storage.
	///
	///  - `&self` :  Self reference
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// # Example:
	/// ``` ignore
	/// curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0",
	/// "id":1, "method":"prices_getAllLockedPrices", "params": []}'
	/// ```
	#[rpc(name = "prices_getAllLockedPrices")]
	fn get_all_locked_prices(&self, at: Option<BlockHash>) -> Result<Vec<(CurrencyId, Option<Price>)>>;
}

/// Struct that implement 'PricesApi'.
pub struct Prices<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> Prices<C, M> {
	/// Create new `Controller` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

// Error enum
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

/// Implementation of 'PricesApi'
impl<C, Block> PricesApi<<Block as BlockT>::Hash> for Prices<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: PricesRuntimeApi<Block>,
{
	fn get_current_price(&self, at: Option<<Block as BlockT>::Hash>, currency_id: CurrencyId) -> Result<Option<Price>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
                // If the block hash is not supplied assume the best block.
                self.client.info().best_hash));

		api.get_current_price(&at, currency_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get price info for the currency.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_all_locked_prices(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(CurrencyId, Option<Price>)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
                // If the block hash is not supplied assume the best block.
                self.client.info().best_hash));

		api.get_all_locked_prices(&at).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get locked prices info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
