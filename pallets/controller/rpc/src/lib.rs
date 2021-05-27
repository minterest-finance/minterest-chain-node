//! RPC interface for the controller pallet.

use codec::Codec;
pub use controller_rpc_runtime_api::{
	BalanceInfo, ControllerApi as ControllerRuntimeApi, HypotheticalLiquidityData, PoolState, UserPoolBalanceData,
};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use minterest_primitives::{CurrencyId, Rate};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait ControllerApi<BlockHash, AccountId> {
	#[rpc(name = "controller_liquidityPoolState")]
	fn liquidity_pool_state(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<PoolState>>;

	#[rpc(name = "controller_utilizationRate")]
	fn get_utilization_rate(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<Rate>>;

	#[rpc(name = "controller_userBalanceInfo")]
	fn get_user_balance(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<UserPoolBalanceData>>;

	#[rpc(name = "controller_accountLiquidity")]
	fn get_hypothetical_account_liquidity(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> Result<Option<HypotheticalLiquidityData>>;

	#[rpc(name = "controller_isAdmin")]
	fn is_admin(&self, caller: AccountId, at: Option<BlockHash>) -> Result<Option<bool>>;

	#[rpc(name = "controller_accountCollateral")]
	fn get_user_total_collateral(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<BalanceInfo>>;

	#[rpc(name = "controller_getUserBorrowPerAsset")]
	fn get_user_borrow_per_asset(
		&self,
		account_id: AccountId,
		underlying_asset_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<BalanceInfo>>;

	#[rpc(name = "controller_poolExists")]
	fn pool_exists(&self, underlying_asset_id: CurrencyId, at: Option<BlockHash>) -> Result<bool>;
}

/// A struct that implements the [`ControllerApi`].
pub struct Controller<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> Controller<C, B> {
	/// Create new `LiquidityPool` with the given reference to the client.
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

impl<C, Block, AccountId> ControllerApi<<Block as BlockT>::Hash, AccountId> for Controller<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: ControllerRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn liquidity_pool_state(
		&self,
		pool_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<PoolState>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.liquidity_pool_state(&at, pool_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get pool state.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_utilization_rate(&self, pool_id: CurrencyId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<Rate>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_utilization_rate(&at, pool_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get pool utilization rate.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_user_balance(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<UserPoolBalanceData>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
		api.get_total_supply_and_borrowed_usd_balance(&at, account_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get balance info.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn get_hypothetical_account_liquidity(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<HypotheticalLiquidityData>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_hypothetical_account_liquidity(&at, account_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get hypothetical account liquidity.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn is_admin(&self, caller: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<bool>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.is_admin(&at, caller).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to check if is an admin.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_user_total_collateral(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<BalanceInfo>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_user_total_collateral(&at, account_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get total user collateral.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_user_borrow_per_asset(
		&self,
		account_id: AccountId,
		underlying_asset_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<BalanceInfo>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_user_borrow_per_asset(&at, account_id, underlying_asset_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get total user borrow balance.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn pool_exists(&self, underlying_asset_id: CurrencyId, at: Option<<Block as BlockT>::Hash>) -> Result<bool> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.pool_exists(&at, underlying_asset_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to check if pool exists.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
