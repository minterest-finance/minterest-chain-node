#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{debug, decl_error, decl_event, decl_module, decl_storage};

use frame_support::traits::Get;
use frame_system::offchain::SendTransactionTypes;
use minterest_primitives::CurrencyId;
use orml_utilities::OffchainErr;
use sp_core::crypto::KeyTypeId;
use sp_runtime::offchain::storage_lock::Time;
use sp_runtime::offchain::Duration;
use sp_runtime::traits::{BlakeTwo256, Hash, Zero};
use sp_runtime::{
	offchain::{storage::StorageValueRef, storage_lock::StorageLock},
	RandomNumberGenerator,
};
use sp_std::{prelude::*, str};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"mint");
pub const NUM_VEC_LEN: usize = 10;

pub const OFFCHAIN_WORKER_DATA: &[u8] = b"pallets/risk-manager/data/";
pub const OFFCHAIN_WORKER_LOCK: &[u8] = b"pallets/risk-manager/lock/";
pub const OFFCHAIN_WORKER_MAX_ITERATIONS: &[u8] = b"pallets/risk-manager/max-iterations/";

pub const LOCK_DURATION: u64 = 100;
pub const DEFAULT_MAX_ITERATIONS: u32 = 1000;

type LiquidityPools<T> = liquidity_pools::Module<T>;

pub trait Trait:
	frame_system::Trait + liquidity_pools::Trait + controller::Trait + oracle::Trait + SendTransactionTypes<Call<Self>>
{
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as RiskManagerStorage {}
}

decl_event!(
	pub enum Event {}
);

decl_error! {
	pub enum Error for Module<T: Trait> {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		fn offchain_worker(now: T::BlockNumber) {
			debug::info!("Entering off-chain worker");

			if let Err(e) = Self::_offchain_worker() {
				debug::info!(
					target: "RiskManager offchain worker",
					"cannot run offchain worker at {:?}: {:?}",
					now,
					e,
				);
			} else {
				debug::debug!(
					target: "RiskManager offchain worker",
					" RiskManager offchain worker start at block: {:?} already done!",
					now,
				);
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn _offchain_worker() -> Result<(), OffchainErr> {
		debug::info!("initial message");

		let underlying_asset_ids: Vec<CurrencyId> = <T as liquidity_pools::Trait>::EnabledCurrencyPair::get()
			.iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();

		if underlying_asset_ids.len().is_zero() {
			return Ok(());
		}

		// check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		// acquire offchain worker lock
		let lock_expiration = Duration::from_millis(LOCK_DURATION);
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		// Get available assets list
		let underlying_asset_ids: Vec<CurrencyId> = <T as liquidity_pools::Trait>::EnabledCurrencyPair::get()
			.iter()
			.map(|currency_pair| currency_pair.underlying_id)
			.collect();

		let to_be_continue = StorageValueRef::persistent(&OFFCHAIN_WORKER_DATA);

		// get to_be_continue record
		let (collateral_position, _start_key) =
			if let Some(Some((last_collateral_position, maybe_last_iterator_previous_key))) =
				to_be_continue.get::<(u32, Option<Vec<u8>>)>()
			{
				(last_collateral_position, maybe_last_iterator_previous_key)
			} else {
				let random_seed = sp_io::offchain::random_seed();
				let mut rng = RandomNumberGenerator::<BlakeTwo256>::new(BlakeTwo256::hash(&random_seed[..]));
				(rng.pick_u32(underlying_asset_ids.len().saturating_sub(1) as u32), None)
			};

		// get the max iterationns config
		let _max_iterations = StorageValueRef::persistent(&OFFCHAIN_WORKER_MAX_ITERATIONS)
			.get::<u32>()
			.unwrap_or(Some(DEFAULT_MAX_ITERATIONS));

		let currency_id = underlying_asset_ids[(collateral_position as usize)];

		// Get pool users list
		let _pool_members = <LiquidityPools<T>>::get_pool_members(currency_id).unwrap();

		// Consume the guard but **do not** unlock the underlying lock.
		guard.forget();

		Ok(())
	}
}
