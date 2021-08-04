use std::sync::Arc;

use minterest_parachain_runtime::{opaque::Block, AccountId, Balance, OriginalAsset, DataProviderId, Index};
pub use sc_rpc::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_transaction_pool::TransactionPool;

/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: orml_oracle_rpc::OracleRuntimeApi<
		Block,
		DataProviderId,
		OriginalAsset,
		minterest_parachain_runtime::TimeStampedPrice,
	>,
	C::Api: controller_rpc::ControllerRuntimeApi<Block, AccountId>,
	C::Api: prices_rpc::PricesRuntimeApi<Block>,
	C::Api: mnt_token_rpc::MntTokenRuntimeApi<Block, AccountId>,
	C::Api: whitelist_rpc::WhitelistRuntimeApi<Block, AccountId>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + 'static,
{
	use controller_rpc::{ControllerRpcApi, ControllerRpcImpl};
	use mnt_token_rpc::{MntTokenRpcApi, MntTokenRpcImpl};
	use orml_oracle_rpc::{Oracle, OracleApi};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use prices_rpc::{PricesRpcApi, PricesRpcImpl};
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	use whitelist_rpc::{WhitelistRpcApi, WhitelistRpcImpl};

	let mut io = jsonrpc_core::IoHandler::default();
	let FullDeps {
		client,
		pool,
		deny_unsafe,
	} = deps;

	io.extend_with(SystemApi::to_delegate(FullSystem::new(
		client.clone(),
		pool,
		deny_unsafe,
	)));

	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
		client.clone(),
	)));

	io.extend_with(ControllerRpcApi::to_delegate(ControllerRpcImpl::new(client.clone())));

	io.extend_with(OracleApi::to_delegate(Oracle::new(client.clone())));

	io.extend_with(MntTokenRpcApi::to_delegate(MntTokenRpcImpl::new(client.clone())));

	io.extend_with(PricesRpcApi::to_delegate(PricesRpcImpl::new(client.clone())));

	io.extend_with(WhitelistRpcApi::to_delegate(WhitelistRpcImpl::new(client)));

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `io.extend_with(YourRpcTrait::to_delegate(YourRpcStruct::new(ReferenceToClient, ...)));`

	io
}
