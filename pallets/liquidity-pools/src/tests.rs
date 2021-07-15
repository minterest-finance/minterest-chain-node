//! Tests for the liquidity-pools module.
use frame_support::{assert_err, assert_noop, assert_ok};
pub use liquidity_pools::Error;
use pallet_traits::{
	Borrowing, CurrencyConverter, LiquidityPoolStorageProvider, PoolsManager, PricesManager, UserStorageProvider,
};
use sp_arithmetic::FixedPointNumber;
use sp_runtime::traits::{One, Zero};
use test_engine::*;

#[test]
fn set_pool_data_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		TestPools::set_pool_data(
			DOT,
			Pool {
				borrowed: ONE_HUNDRED,
				borrow_index: Rate::saturating_from_rational(125, 100),
				protocol_interest: ONE_HUNDRED,
			},
		);
		assert_eq!(TestPools::get_pool_data(DOT).borrowed, ONE_HUNDRED);
		assert_eq!(
			TestPools::get_pool_data(DOT).borrow_index,
			Rate::saturating_from_rational(125, 100)
		);
		assert_eq!(TestPools::get_pool_data(DOT).protocol_interest, ONE_HUNDRED);
	});
}

#[test]
fn set_pool_borrow_underlying_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// Set pool_borrowed eq 100 DOT
		TestPools::set_pool_borrow_underlying(DOT, ONE_HUNDRED);
		assert_eq!(TestPools::get_pool_data(DOT).borrowed, ONE_HUNDRED);
	});
}

#[test]
fn set_pool_protocol_interest_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// Set pool_protocol_interest eq 100 DOT.
		TestPools::set_pool_protocol_interest(DOT, ONE_HUNDRED);
		assert_eq!(TestPools::get_pool_data(DOT).protocol_interest, ONE_HUNDRED);
	});
}

#[test]
fn set_user_borrow_and_interest_index_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// Set user_borrowed eq 100 DOT and user_interest_index eq 0.33.
		TestPools::set_user_borrow_and_interest_index(
			&ALICE,
			DOT,
			ONE_HUNDRED,
			Rate::saturating_from_rational(33, 100),
		);
		assert_eq!(TestPools::get_user_data(DOT, &ALICE).borrowed, ONE_HUNDRED);
		assert_eq!(
			TestPools::pool_user_data(DOT, ALICE).interest_index,
			Rate::saturating_from_rational(33, 100)
		);
	});
}

#[test]
fn enable_is_collateral_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// Alice enable as collateral DOT pool.
		TestPools::enable_is_collateral(&ALICE, DOT);

		assert!(TestPools::pool_user_data(DOT, ALICE).is_collateral);
	});
}

#[test]
fn enable_is_collateral_internal_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// Alice disable collateral DOT pool.
		TestPools::disable_is_collateral(&ALICE, DOT);

		assert!(!TestPools::pool_user_data(DOT, ALICE).is_collateral);
	});
}

#[test]
fn get_pool_available_liquidity_should_work() {
	ExtBuilderNew::default()
		.set_pool_balance(DOT, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_available_liquidity(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_data_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,                                      // pool_id
			TEN_THOUSAND,                             // borrowed
			Rate::saturating_from_rational(125, 100), // borrow_index
			TEN_THOUSAND,                             // protocol_interest
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::get_pool_data(DOT),
				Pool {
					borrowed: TEN_THOUSAND,
					borrow_index: Rate::saturating_from_rational(125, 100),
					protocol_interest: TEN_THOUSAND,
				}
			);
		});
}

#[test]
fn get_pool_borrow_underlying_should_work() {
	ExtBuilderNew::default()
		.init_pool(DOT, TEN_THOUSAND, Rate::default(), Balance::default())
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_protocol_interest_should_work() {
	ExtBuilderNew::default()
		.init_pool(DOT, Balance::default(), Rate::default(), TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_protocol_interest(DOT), TEN_THOUSAND);
		});
}

#[test]
fn get_pool_borrow_index_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,
			Balance::default(),
			Rate::saturating_from_rational(125, 100),
			Balance::default(),
		)
		.build()
		.execute_with(|| {
			assert_eq!(
				TestPools::get_pool_borrow_index(DOT),
				Rate::saturating_from_rational(125, 100)
			);
		});
}

#[test]
fn get_user_borrow_balance_should_work() {
	ExtBuilderNew::default()
		.set_pool_user_data(
			DOT,             // pool_id
			ALICE,           // user
			ONE_HUNDRED,     // borrowed
			Rate::default(), // interest_index
			true,            // is_collateral
		)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), ONE_HUNDRED);
		});
}

#[test]
fn check_user_available_is_collateral_should_work() {
	ExtBuilderNew::default()
		.set_pool_user_data(
			DOT,                // pool_id
			ALICE,              // user
			Balance::default(), // borrowed
			Rate::default(),    // interest_index
			false,              // is_collateral
		)
		.build()
		.execute_with(|| {
			// collateral parameter is set to false
			assert!(!TestPools::is_pool_collateral(&ALICE, DOT));

			// set collateral parameter to true
			TestPools::enable_is_collateral(&ALICE, DOT);

			assert!(TestPools::is_pool_collateral(&ALICE, DOT));
		});
}

#[test]
fn pool_should_exists() {
	ExtBuilderNew::default()
		.init_pool_default(DOT)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::pool_exists(&DOT), true);
			assert_eq!(TestPools::pool_exists(&MDOT), false);
		});
}

#[test]
fn update_state_on_borrow_should_work() {
	ExtBuilderNew::default()
		.set_user_balance(ALICE, DOT, ONE_HUNDRED)
		.init_pool_default(DOT)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::from_inner(0));

			// Alice borrow 60 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 60, 0));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), 60);
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), 60);
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::default());

			let mut dot_pool_data = TestPools::get_pool_data(DOT);
			dot_pool_data.borrow_index = Rate::saturating_from_rational(1, 5);
			TestPools::set_pool_data(DOT, dot_pool_data);

			// ALice borrow 30 DOT
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, 30, 60));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), 90);
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), 90);
			assert_eq!(
				TestPools::get_user_borrow_index(&ALICE, DOT),
				Rate::saturating_from_rational(1, 5)
			);

			// Overflow in calculation: account_borrow_new = 90 + max_value()
			assert_noop!(
				TestPools::update_state_on_borrow(&ALICE, DOT, Balance::max_value(), 90),
				Error::<TestRuntime>::BorrowBalanceOverflow
			);
		});
}

#[test]
fn update_state_on_repay_should_work() {
	ExtBuilderNew::default()
		.set_user_balance(ALICE, DOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::from_inner(0));
			assert_ok!(TestPools::update_state_on_borrow(&ALICE, DOT, dollars(60), 0));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), dollars(60));
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), dollars(60));
			assert_eq!(TestPools::get_user_borrow_index(&ALICE, DOT), Rate::default());

			assert_ok!(TestPools::update_state_on_repay(&ALICE, DOT, dollars(30), dollars(60)));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), dollars(30));
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), dollars(30));

			assert_ok!(TestPools::update_state_on_repay(&ALICE, DOT, dollars(10), dollars(30)));
			assert_eq!(TestPools::get_pool_borrow_underlying(DOT), dollars(20));
			assert_eq!(TestPools::get_user_borrow_balance(&ALICE, DOT), dollars(20));

			assert_noop!(
				TestPools::update_state_on_repay(&ALICE, DOT, 100, 20),
				Error::<TestRuntime>::RepayAmountTooBig
			);
		});
}

#[test]
fn get_user_liquidation_attempts_should_work() {
	ExtBuilderNew::default()
		.set_pool_user_data(
			DOT,             // pool_id
			ALICE,           // user
			ONE_HUNDRED,     // borrowed
			Rate::default(), // interest_index
			true,            // is_collateral
			12,              // liquidation_attempts
		)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_liquidation_attempts(&ALICE, DOT), 12);
		});
}

#[test]
fn get_pool_members_with_loans_should_work() {
	ExtBuilderNew::default()
		.set_pool_user_data(DOT, ALICE, ONE_HUNDRED, Rate::default(), true)
		.set_pool_user_data(DOT, BOB, 0, Rate::default(), true)
		.set_pool_user_data(DOT, CHARLIE, 100, Rate::default(), true)
		.set_pool_user_data(BTC, ALICE, 0, Rate::default(), true)
		.set_pool_user_data(BTC, BOB, 0, Rate::default(), true)
		.set_pool_user_data(BTC, CHARLIE, ONE_HUNDRED, Rate::default(), true)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_pool_members_with_loans(DOT), Ok(vec![CHARLIE, ALICE]));
			assert_eq!(TestPools::get_pool_members_with_loans(BTC), Ok(vec![CHARLIE]));
		});
}

#[test]
fn check_user_has_collateral_should_work() {
	ExtBuilderNew::default()
		.init_pool_default(DOT)
		.init_pool_default(BTC)
		.init_pool_default(ETH)
		.set_pool_user_data(DOT, ALICE, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(BTC, ALICE, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(ETH, ALICE, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(DOT, BOB, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(BTC, CHARLIE, Balance::zero(), Rate::default(), false)
		.set_user_balance(ALICE, MDOT, Balance::zero())
		.set_user_balance(ALICE, MBTC, Balance::zero())
		.set_user_balance(ALICE, METH, TEN_THOUSAND)
		.set_user_balance(BOB, MDOT, Balance::zero())
		.set_user_balance(CHARLIE, MBTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::check_user_has_collateral(&ALICE), true);
			assert_eq!(TestPools::check_user_has_collateral(&BOB), false);
			assert_eq!(TestPools::check_user_has_collateral(&CHARLIE), false);
		});
}

#[test]
fn calculate_exchange_rate_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		// exchange_rate = (102 - 2 + 20) / 100 = 1.2
		assert_eq!(
			TestPools::calculate_exchange_rate(102, 100, 2, 20),
			Ok(Rate::saturating_from_rational(12, 10))
		);
		// If there are no tokens minted: exchangeRate = InitialExchangeRate = 1.0
		assert_eq!(
			TestPools::calculate_exchange_rate(102, 0, 2, 0),
			Ok(Rate::saturating_from_rational(1, 1))
		);

		// Overflow in calculation: pool_supply_underlying + pool_borrow_underlying
		assert_noop!(
			TestPools::calculate_exchange_rate(Balance::max_value(), 100, 100, 100),
			Error::<TestRuntime>::ExchangeRateCalculationError
		);

		// Overflow in calculation: pool_supply_underlying + pool_borrow_underlying -
		// - pool_protocol_interest
		assert_noop!(
			TestPools::calculate_exchange_rate(100, 100, Balance::max_value(), 100),
			Error::<TestRuntime>::ExchangeRateCalculationError
		);
	});
}

#[test]
fn get_user_collateral_pools_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			KSM,             // pool_id
			ONE_HUNDRED,     // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.init_pool(
			DOT,             // pool_id
			ONE_HUNDRED,     // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.init_pool(
			ETH,             // pool_id
			ONE_HUNDRED,     // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.init_pool(
			BTC,             // pool_id
			ONE_HUNDRED,     // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.set_pool_balance(KSM, 1 * TEN_THOUSAND)
		.set_pool_balance(DOT, 3 * TEN_THOUSAND)
		.set_pool_balance(ETH, 2 * TEN_THOUSAND)
		.set_pool_balance(BTC, 4 * TEN_THOUSAND)
		.set_pool_user_data(KSM, ALICE, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(DOT, ALICE, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(ETH, ALICE, Balance::zero(), Rate::default(), true)
		.set_pool_user_data(BTC, ALICE, Balance::zero(), Rate::default(), false)
		.set_user_balance(ALICE, MKSM, Balance::zero())
		.set_user_balance(ALICE, MDOT, TEN_THOUSAND)
		.set_user_balance(ALICE, METH, TEN_THOUSAND)
		.set_user_balance(ALICE, MBTC, TEN_THOUSAND)
		.build()
		.execute_with(|| {
			assert_eq!(TestPools::get_user_collateral_pools(&ALICE), Ok(vec![DOT, ETH]));
			assert_eq!(TestPools::get_user_collateral_pools(&BOB), Ok(vec![]));
		});
}

#[test]
fn increase_and_reset_user_liquidation_attempts_should_work() {
	ExtBuilderNew::default().build().execute_with(|| {
		TestPools::increase_user_liquidation_attempts(DOT, &ALICE);
		assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, u8::one());
		TestPools::increase_user_liquidation_attempts(DOT, &ALICE);
		assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, 2_u8);
		TestPools::reset_user_liquidation_attempts(DOT, &ALICE);
		assert_eq!(TestPools::pool_user_data(DOT, ALICE).liquidation_attempts, u8::zero());
	})
}

// Currency converter tests
#[test]
fn get_exchange_rate_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,               // pool_id
			dollars(300_u128), // borrowed
			Rate::one(),       // borrow_index
			Balance::zero(),   // protocol_interest
		)
		.set_pool_balance(DOT, dollars(100_u128))
		.set_user_balance(ALICE, MDOT, dollars(125_u128))
		.build()
		.execute_with(|| {
			// Pool needs to be created first
			assert_noop!(TestPools::get_exchange_rate(ETH), Error::<TestRuntime>::PoolNotFound);
			// exchange_rate = (100 - 0 + 300) / 125 = 3.2
			assert_eq!(
				TestPools::get_exchange_rate(DOT),
				Ok(Rate::saturating_from_rational(32, 10))
			);
		});
}

#[test]
fn underlying_to_wrapped_and_usd_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			DOT,             // pool_id
			dollars(40),     // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.set_user_balance(ALICE, DOT, ONE_HUNDRED)
		.set_user_balance(ALICE, MDOT, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			// exchange_rate = 40 / 100 = 0.4
			let exchange_rate = TestPools::get_exchange_rate(DOT).unwrap();
			assert_eq!(TestPools::underlying_to_wrapped(10, exchange_rate), Ok(25));

			// Overflow in calculation: wrapped_amount = max_value() / exchange_rate,
			// when exchange_rate < 1
			assert_err!(
				TestPools::underlying_to_wrapped(Balance::max_value(), exchange_rate),
				Error::<TestRuntime>::ConversionError
			);

			// oracle_price = 2 USD.
			let oracle_price = MockPriceSource::get_underlying_price(DOT).unwrap();
			assert_eq!(TestPools::underlying_to_usd(10, oracle_price), Ok(20));

			assert_eq!(TestPools::usd_to_underlying(20, oracle_price), Ok(10));
		});
}

#[test]
fn wrapped_to_underlying_and_usd_should_work() {
	ExtBuilderNew::default()
		.init_pool(
			BTC,             // pool_id
			Balance::zero(), // borrowed
			Rate::zero(),    // borrow_index
			Balance::zero(), // protocol_interest
		)
		.init_pool(
			DOT,             // pool_id
			dollars(40),     // borrowed
			Rate::one(),     // borrow_index
			Balance::zero(), // protocol_interest
		)
		.set_user_balance(ALICE, DOT, ONE_HUNDRED)
		.set_user_balance(ALICE, MDOT, ONE_HUNDRED)
		.set_user_balance(ALICE, MBTC, 1)
		.set_pool_balance(BTC, ONE_HUNDRED)
		.build()
		.execute_with(|| {
			// Set price = 2.00 USD for all assets.
			MockPriceSource::set_underlying_price(Some(Price::from_inner(2 * DOLLARS)));

			let exchange_rate_dot = TestPools::get_exchange_rate(DOT).unwrap();
			// underlying_amount = 10 * 0.4 = 4
			assert_eq!(TestPools::wrapped_to_underlying(10, exchange_rate_dot), Ok(4));

			// Overflow in calculation: underlying_amount = max_value() * exchange_rate
			let exchange_rate_btc = TestPools::get_exchange_rate(BTC).unwrap();
			assert_err!(
				TestPools::wrapped_to_underlying(Balance::max_value(), exchange_rate_btc),
				Error::<TestRuntime>::ConversionError
			);

			// oracle_price = 2 USD.
			let oracle_price = MockPriceSource::get_underlying_price(DOT).unwrap();
			assert_eq!(TestPools::wrapped_to_usd(10, exchange_rate_dot, oracle_price), Ok(8));

			// wrapped_amount = 20 / 2 / 0.4 = 25
			assert_eq!(TestPools::usd_to_wrapped(20, exchange_rate_dot, oracle_price), Ok(25));
		});
}
