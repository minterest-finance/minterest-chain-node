#[macro_export]
macro_rules! impl_system_config {
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
