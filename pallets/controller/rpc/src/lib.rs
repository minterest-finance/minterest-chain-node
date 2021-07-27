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
	/// Parameters:
	///  - `&self`: Self reference
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`pool_total_supply_in_usd`](`ProtocolTotalValue::pool_total_supply_in_usd`):
	/// total available liquidity in the protocol in usd.
	/// - [`pool_total_borrow_in_usd`](`ProtocolTotalValue::pool_total_borrow_in_usd`):
	/// total borrowed including interest in the protocol in usd.
	/// - [`tvl_in_usd`](`ProtocolTotalValue::tvl_in_usd`): total value of locked money in protocol
	///   in usd.
	/// - [`pool_total_protocol_interest_in_usd`](`ProtocolTotalValue::
	/// pool_total_protocol_interest_in_usd`): total protocol interest for all pools in usd.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_protocolTotalValues")]
	fn get_protocol_total_values(&self, at: Option<BlockHash>) -> Result<Option<ProtocolTotalValue>>;

	/// Returns current Liquidity Pool State.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `pool_id`: target pool id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	///	is equal to:
	///
	/// `exchange_rate = (pool_supply_underlying + pool_borrow_underlying - pool_protocol_interest)
	/// / pool_supply;`
	///
	/// Return:
	/// - [`exchange_rate`](`PoolState::exchange_rate`): the Exchange Rate between an mToken and the
	/// underlying asset.
	/// - [`borrow_rate`](`PoolState::borrow_rate`): Borrow Interest Rate
	/// - [`supply_rate`](`PoolState::supply_rate`): Supply Interest Rate.
	///  The supply rate is derived from the borrow_rate and utilization_rate.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_liquidityPoolState")]
	fn liquidity_pool_state(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<PoolState>>;

	/// Returns utilization rate based on pool parameters calculated for current block.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `pool_id`: target pool id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - utilization_rate: current utilization rate of a pool.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_utilizationRate")]
	fn get_pool_utilization_rate(&self, pool_id: CurrencyId, at: Option<BlockHash>) -> Result<Option<Rate>>;

	/// Returns user total supply and user total borrowed balance in usd.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: target account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`total_supply`](`UserPoolBalanceData::total_supply`): total balance that user has in all
	/// pools converted to usd
	/// - [`total_borrowed`](`UserPoolBalanceData::total_borrowed`): user total borrowed underlying
	/// assets from all
	/// pools converted to usd.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_userBalanceInfo")]
	fn get_user_balance(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<UserPoolBalanceData>>;

	/// Returns account liquidity in usd.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: target account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`liquidity`](`HypotheticalLiquidityData::liquidity`): account liquidity in usd.
	/// Positive amount if user has collateral greater than borrowed, otherwise negative.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_accountLiquidity")]
	fn get_hypothetical_account_liquidity(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> Result<Option<HypotheticalLiquidityData>>;

	/// Checks whether the caller is a member of the MinterestCouncil.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return
	/// - `is_admin`: true / false
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_isAdmin")]
	fn is_admin(&self, caller: AccountId, at: Option<BlockHash>) -> Result<Option<bool>>;

	/// Returns user total collateral in usd.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`amount`](`BalanceInfo::amount`): user total collateral in all liquidity pools converted
	/// to usd.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_accountCollateral")]
	fn get_user_total_collateral(&self, account_id: AccountId, at: Option<BlockHash>) -> Result<Option<BalanceInfo>>;

	/// Returns actual borrow underlying balance for user per asset based on fresh latest indexes.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `underlying_asset_id`: current asset id
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - [`amount`](`BalanceInfo::amount`): user borrow underlying in a specific liquidity pool
	/// (underlying assets amount).
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_getUserBorrowPerAsset")]
	fn get_user_borrow_per_asset(
		&self,
		account_id: AccountId,
		underlying_asset_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<BalanceInfo>>;

	/// Returns actual supply underlying balance for user in specific liquidity pool based on fresh
	/// latest indexes.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `pool_id`: target pool id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	///  Return:
	///  - [`amount`](`BalanceInfo::amount`): user supply underlying in a specific liquidity pool
	/// (underlying assets amount).
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_getUserUnderlyingBalancePerAsset")]
	fn get_user_underlying_balance_per_asset(
		&self,
		account_id: AccountId,
		pool_id: CurrencyId,
		at: Option<BlockHash>,
	) -> Result<Option<BalanceInfo>>;

	/// Checks whether the pool is created in storage.
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `underlying_asset_id`: current asset id
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	/// - is_created: true / false
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_poolExists")]
	fn pool_exists(&self, underlying_asset_id: CurrencyId, at: Option<BlockHash>) -> Result<bool>;

	/// Return borrow APY, supply APY and Net APY for current user
	///
	/// Parameters:
	///  - `&self`: Self reference
	///  - `account_id`: current account id.
	///  - `at`: Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Return:
	///
	/// (supply_apy,borrow_apy,net_apy)
	///
	/// - [`supply_apy`](`minterest_primitives::Interest`): supply APY value for the user.
	/// - [`borrow_apy`](`minterest_primitives::Interest`): borrow APY value for the user.
	/// - [`net_apy`](`minterest_primitives::Interest`): net APY value for the user.
	#[doc(alias("MNT RPC", "MNT controller"))]
	#[rpc(name = "controller_getUserTotalSupplyBorrowAndNetApy")]
	fn get_user_total_supply_borrow_and_net_apy(
		&self,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> Result<Option<(Interest, Interest, Interest)>>;

	// TODO: Currently all parameters are stabbed to equal to one.
	/// Return user's information which is required by WEB 2.0 part.
	///
	/// Parameters:
	///  - `&self` :  Self reference
	///  - `account_id`: current account id.
	///  - `at` : Needed for runtime API use. Runtime API must always be called at a specific block.
	///
	/// Returns:
    ///
	/// (user_total_collateral, user_total_supply_in_usd, user_total_borrow_in_usd,
	///   user_total_supply_apy, user_total_borrow_apy, user_net_apy)
    ///
	/// - [`total_collateral_in_usd`](`UserData::total_collateral_in_usd`): user total collateral
	/// in usd.
	/// - [`total_supply_in_usd`](`UserData::total_supply_in_usd`): user total supplied to the
	/// protocol in usd.
	/// - [`total_borrow_in_usd`](`UserData::total_borrow_in_usd`): user total borrowed including
	/// interest in usd.
	/// - [`total_supply_apy`](`UserData::total_supply_apy`): user total supply apy value.
	/// - [`total_borrow_apy`](`UserData::total_borrow_apy`): user total borrow apy value.
	/// - [`net_apy`](`UserData::net_apy`): user net APY value.
	#[doc(alias("MNT RPC", "MNT controller"))]
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

	fn get_pool_utilization_rate(
		&self,
		pool_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<Rate>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_pool_utilization_rate(&at, pool_id).map_err(|e| RpcError {
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
