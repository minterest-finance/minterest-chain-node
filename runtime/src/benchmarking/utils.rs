use crate::{
	AccountId, Balance, Currencies, CurrencyId, EnabledUnderlyingAssetId, MinterestOracle, MinterestProtocol, Origin,
	Price, Runtime, Vec,
};

use frame_support::pallet_prelude::DispatchResultWithPostInfo;
use frame_support::traits::OnFinalize;
use frame_system::pallet_prelude::OriginFor;
use orml_traits::MultiCurrency;
use sp_runtime::{traits::StaticLookup, FixedPointNumber};

pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Config>::Lookup::unlookup(who)
}

pub fn set_oracle_price_for_all_pools<T: frame_system::Config<Origin = Origin>>(
	price: u128,
	origin: OriginFor<T>,
	block: u32,
) -> DispatchResultWithPostInfo {
	let prices: Vec<(CurrencyId, Price)> = EnabledUnderlyingAssetId::get()
		.into_iter()
		.map(|pool_id| (pool_id, Price::saturating_from_integer(price)))
		.collect();
	MinterestOracle::on_finalize(block);
	MinterestOracle::feed_values(origin.into(), prices)?;
	Ok(().into())
}

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) -> DispatchResultWithPostInfo {
	<Currencies as MultiCurrency<_>>::deposit(currency_id, &who, balance)?;
	Ok(().into())
}

pub fn enable_as_collateral<T: frame_system::Config<Origin = Origin>>(
	origin: OriginFor<T>,
	currency_id: CurrencyId,
) -> DispatchResultWithPostInfo {
	MinterestProtocol::enable_as_collateral(origin.into(), currency_id)?;
	Ok(().into())
}
