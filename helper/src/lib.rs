#[macro_export]
macro_rules! mock_impl_system_config {
    ($target:ty) => {
        parameter_types! {
            pub const BlockHashCount: u64 = 250;
            pub const SS58Prefix: u8 = 42;
        }

        impl system::Config for $target {
            type BaseCallFilter = ();
            type BlockWeights = ();
            type BlockLength = ();
            type DbWeight = ();
            type Origin = Origin;
            type Call = Call;
            type Index = u64;
            type BlockNumber = u64;
            type Hash = H256;
            type Hashing = BlakeTwo256;
            type AccountId = u64;
            type Lookup = IdentityLookup<Self::AccountId>;
            type Header = Header;
            type Event = Event;
            type BlockHashCount = BlockHashCount;
            type Version = ();
            type PalletInfo = PalletInfo;
            type AccountData = ();
            type OnNewAccount = ();
            type OnKilledAccount = ();
            type SystemWeightInfo = ();
            type SS58Prefix = SS58Prefix;
        }
    }
}

#[macro_export]
macro_rules! mock_impl_orml_tokens_config {
    ($target:ty) => {
        parameter_type_with_key! {
            pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
                Default::default()
            };
        }

        impl orml_tokens::Config for $target {
            type Event = Event;
            type Balance = Balance;
            type Amount = Amount;
            type CurrencyId = CurrencyId;
            type WeightInfo = ();
            type ExistentialDeposits = ExistentialDeposits;
            type OnDust = ();
        }
    }
}

#[macro_export]
macro_rules! mock_impl_orml_currencies_config {
    ($target:ty, $currency_id:expr) => {
        parameter_types! {
            pub const GetNativeCurrencyId: CurrencyId = $currency_id;
        }

        type NativeCurrency = Currency<$target, GetNativeCurrencyId>;

        impl orml_currencies::Config for $target {
            type Event = Event;
            type MultiCurrency = orml_tokens::Module<$target>;
            type NativeCurrency = NativeCurrency;
            type GetNativeCurrencyId = GetNativeCurrencyId;
            type WeightInfo = ();
        }
    }
}

#[macro_export]
macro_rules! mock_impl_liquidity_pools_config {
    ($target:ty) => {
        impl liquidity_pools::Config for $target {
            type MultiCurrency = orml_tokens::Module<$target>;
            type PriceSource = MockPriceSource;
            type ModuleId = LiquidityPoolsModuleId;
            type LiquidityPoolAccountId = LiquidityPoolAccountId;
            type InitialExchangeRate = InitialExchangeRate;
            type EnabledCurrencyPair = EnabledCurrencyPair;
            type EnabledUnderlyingAssetId = EnabledUnderlyingAssetId;
            type EnabledWrappedTokensId = EnabledWrappedTokensId;
        }
    }
}
