#[macro_export]
macro_rules! mock_impl_system_config {
    ($target:ty) => {
        parameter_types! {
            pub const MockBlockHashCount: u64 = 250;
            pub const MockSS58Prefix: u8 = 42;
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
            type BlockHashCount = MockBlockHashCount;
            type Version = ();
            type PalletInfo = PalletInfo;
            type AccountData = ();
            type OnNewAccount = ();
            type OnKilledAccount = ();
            type SystemWeightInfo = ();
            type SS58Prefix = MockSS58Prefix;
        }
    }
}

#[macro_export]
macro_rules! mock_impl_orml_tokens_config {
    ($target:ty) => {
        parameter_type_with_key! {
            pub MockExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
                Default::default()
            };
        }

        impl orml_tokens::Config for $target {
            type Event = Event;
            type Balance = Balance;
            type Amount = Amount;
            type CurrencyId = CurrencyId;
            type WeightInfo = ();
            type ExistentialDeposits = MockExistentialDeposits;
            type OnDust = ();
        }
    }
}

#[macro_export]
macro_rules! mock_impl_orml_currencies_config {
    ($target:ty, $currency_id:expr) => {
        parameter_types! {
            pub const MockGetNativeCurrencyId: CurrencyId = $currency_id;
        }

        type MockNativeCurrency = Currency<$target, MockGetNativeCurrencyId>;

        impl orml_currencies::Config for $target {
            type Event = Event;
            type MultiCurrency = orml_tokens::Module<$target>;
            type NativeCurrency = MockNativeCurrency;
            type GetNativeCurrencyId = MockGetNativeCurrencyId;
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

#[macro_export]
macro_rules! mock_impl_liquidation_pools_config {
    ($target:ty) => {
        parameter_types! {
            pub const MockLiquidityPoolsPriority: TransactionPriority = TransactionPriority::max_value() - 1;
        }

        impl liquidation_pools::Config for $target {
            type Event = Event;
            type UnsignedPriority = MockLiquidityPoolsPriority;
            type LiquidationPoolsModuleId = LiquidationPoolsModuleId;
            type LiquidationPoolAccountId = LiquidationPoolAccountId;
            type LiquidityPoolsManager = liquidity_pools::Module<$target>;
            type UpdateOrigin = EnsureSignedBy<ZeroAdmin, AccountId>;
            type Dex = dex::Module<$target>;
            type LiquidationPoolsWeightInfo = ();
        }

        /// An extrinsic type used for tests.
        pub type MockExtrinsic = TestXt<Call, ()>;

        impl<LocalCall> SendTransactionTypes<LocalCall> for $target
        where
            Call: From<LocalCall>,
        {
            type OverarchingCall = Call;
            type Extrinsic = MockExtrinsic;
        }
    }
}
