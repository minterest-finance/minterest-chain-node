//! RPC interface for the controller pallet.

use codec::Codec;
use controller_rpc_runtime_api::UserData;
pub use controller_rpc_runtime_api::{
	BalanceInfo, ControllerRuntimeApi, HypotheticalLiquidityData, PoolState, ProtocolTotalValue, UserPoolBalanceData,
};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use minterest_primitives::{CurrencyId, Interest, Rate};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
/// Base trait for RPC interface of controller
pub trait ControllerRpcApi<BlockHash, AccountId> {
	/// Returns total values of supply, borrow, locked and protocol_interest.
	///
	///  - `&self`: Self reference
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`pool_total_supply_in_usd`](`ProtocolTotalValue::pool_total_supply_in_usd`): total available liquidity in the protocol in usd.
	/// - [`pool_total_borrow_in_usd`](`ProtocolTotalValue::pool_total_borrow_in_usd`): total borrowed including interest in the protocol in usd.
	/// - [`tvl_in_usd`](`ProtocolTotalValue::tvl_in_usd`): total value of locked money in protocol in usd.
	/// - [`pool_total_protocol_interest_in_usd`](`ProtocolTotalValue::pool_total_protocol_interest_in_usd`): total protocol interest for all pools in usd.
	#[rpc(name = "controller_protocolTotalValues")]
	fn get_protocol_total_values(&self, at: Option<BlockHash>) -> Result<Option<ProtocolTotalValue>>;

	/// Returns current Liquidity Pool State.
	///
	///  - `&self`: Self reference
	///  - `pool_id`: current pool id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	///	is equal to:
	/// `exchange_rate = (pool_supply_underlying + pool_borrow_underlying - pool_protocol_interest) /
	/// pool_supply;`
	///
	/// Return:
	/// - [`exchange_rate`](`PoolState::exchange_rate`): the Exchange Rate between an mToken and the underlying asset.
	/// - [`borrow_rate`](`PoolState::borrow_rate`): Borrow Interest Rate
	/// - [`supply_rate`](`PoolState::supply_rate`): current Supply Interest Rate.
	///  The supply rate is derived from the borrow_rate and the amount of Total Borrowed.
	#[rpc(name = "controller_liquidityPoolState")]
	fn liquidity_pool_state(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<PoolState>>;

	/// Returns utilization rate based on pool parameters calculated for current block.
	///
	///  - `&self`: Self reference
	///  - `pool_id`: target pool id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - utilization_rate: current utilization rate of a pool.
	#[rpc(name = "controller_utilizationRate")]
	fn get_utilization_rate(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<Rate>>;

	/// Returns total supply and total borrowed balance in usd.
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`total_supply`](`UserPoolBalanceData::total_supply`): total balance that user has in all pools converted to usd
	/// - [`total_borrowed`](`UserPoolBalanceData::total_borrowed`): total borrowed tokens from all pools converted to usd.
	#[rpc(name = "controller_userBalanceInfo")]
	fn get_user_balance(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<UserPoolBalanceData>>;

	/// Returns account liquidity in usd.
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`liquidity`](`HypotheticalLiquidityData::liquidity`): account liquidity in usd.
	/// Positive amount if user has collateral greater than borrowed, otherwise negative.
	#[rpc(name = "controller_accountLiquidity")]
	fn get_hypothetical_account_liquidity(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> Result<Option<HypotheticalLiquidityData>>;

	/// Checks whether the caller is a member of the MinterestCouncil.
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return
	/// - `is_admin`: true / false
	#[rpc(name = "controller_isAdmin")]
	fn is_admin(&self, caller: AccountId, at: Option<BlockHash>) -> Result<Option<bool>>;

	/// Returns account total collateral in usd.
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`amount`](`BalanceInfo::amount`): account total collateral converted to usd.
	#[rpc(name = "controller_accountCollateral")]
	fn get_user_total_collateral(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<BalanceInfo>>;

	/// Returns actual borrow balance for user per asset based on fresh latest indexes.
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `underlying_asset_id`: current asset id
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`amount`](`BalanceInfo::amount`): account total borrow per asset.
	#[rpc(name = "controller_getUserBorrowPerAsset")]
	fn get_user_borrow_per_asset(
		&self,
		account_id: AccountId,
		underlying_asset_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<BalanceInfo>>;

	/// Returns user underlying asset balance for the pool
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `pool_id`: target pool id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	///  Return:
	///  - [`amount`](`BalanceInfo::amount`): account supply underlying assets balance in liquidity pool.
	#[rpc(name = "controller_getUserUnderlyingBalancePerAsset")]
	fn get_user_underlying_balance_per_asset(
		&self,
		account_id: AccountId,
		pool_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<BalanceInfo>>;

	/// Checks whether the pool is created in storage.
	///
	///  - `&self`: Self reference
	///  - `underlying_asset_id`: current asset id
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - is_created: true / false
	#[rpc(name = "controller_poolExists")]
	fn pool_exists(&self, underlying_asset_id: CurrencyId, at: Option<BlockHash>) -> Result<bool>;

	/// Return borrow APY, supply APY and Net APY for current user
	///
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	///
	/// (supply_apy,borrow_apy,net_apy)
	///
	/// - [`supply_apy`](`minterest_primitives::Interest`):  supply APY
	/// - [`borrow_apy`](`minterest_primitives::Interest`): borrow APY
	/// - [`net_apy`](`minterest_primitives::Interest`):  net APY
	#[rpc(name = "controller_getUserTotalSupplyBorrowAndNetApy")]
	fn get_user_total_supply_borrow_and_net_apy(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> Result<Option<(Interest, Interest, Interest)>>;

	// TODO: Currently all parameters are stabbed to equal to one.
	/// Return user's information which is required by WEB 2.0 part.
	///
	///  - `&self` :  Self reference
	///  - `account_id`: current account id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Returns:
	/// - [`total_collateral_in_usd`](`UserData::total_collateral_in_usd`): account total collateral converted to usd.
	/// - [`total_supply_in_usd`](`UserData::total_supply_in_usd`): account total supply converted to usd.
	/// - [`total_borrow_in_usd`](`UserData::total_borrow_in_usd`): account total borrow converted to usd.
	/// - [`total_supply_apy`](`UserData::total_supply_apy`): account total supply APY
	/// - [`total_borrow_apy`](`UserData::total_borrow_apy`): account total borrow APY
	/// - [`net_apy`](`UserData::net_apy`): account net APY
	#[rpc(name = "controller_getUserData")]
	fn get_user_data(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<UserData>>;
}

/// A struct that implements the [`ControllerApi`].
pub struct ControllerRpcImpl<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> ControllerRpcImpl<C, B> {
	/// Create new `ControllerRpcImpl` with the given reference to the client.
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

/// Implementation of 'ControllerRpcApi'
impl<C, Block, AccountId> ControllerRpcApi<<Block as BlockT>::Hash, AccountId> for ControllerRpcImpl<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: ControllerRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn get_user_data(&self, account_id: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<UserData>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_user_data(&at, account_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get user data.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_protocol_total_values(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Option<ProtocolTotalValue>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_protocol_total_values(&at).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get protocol total values.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

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
		api.get_user_total_supply_and_borrow_balance_in_usd(&at, account_id)
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

	fn get_user_underlying_balance_per_asset(
		&self,
		account_id: AccountId,
		pool_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<BalanceInfo>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_user_underlying_balance_per_asset(&at, account_id, pool_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get user underlying balance.".into(),
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

	fn get_user_total_supply_borrow_and_net_apy(
		&self,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<(Interest, Interest, Interest)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_user_total_supply_borrow_and_net_apy(&at, account_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get user's APY.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}
}
