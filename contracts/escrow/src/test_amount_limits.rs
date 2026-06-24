#![cfg(test)]
extern crate std;

use crate::{ContractError, Escrow, EscrowClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

struct Fixture {
    env: Env,
    client: EscrowClient<'static>,
    token: token::StellarAssetClient<'static>,
    token_admin: token::StellarAssetClient<'static>,
    admin: Address,
    seller: Address,
    buyer: Address,
    resolver: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register_contract(None, Escrow);
        let client = EscrowClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let fee_collector = Address::generate(&env);
        client.initialize(&admin, &fee_collector, &500);

        let token_admin = Address::generate(&env);
        let token_addr = env.register_stellar_asset_contract(token_admin.clone());
        let token = token::StellarAssetClient::new(&env, &token_addr);
        let token_admin_client = token::StellarAssetClient::new(&env, &token_addr);

        let seller = Address::generate(&env);
        let buyer = Address::generate(&env);
        let resolver = Address::generate(&env);

        Self {
            env,
            client,
            token,
            token_admin: token_admin_client,
            admin,
            seller,
            buyer,
            resolver,
        }
    }
}

#[test]
fn test_valid_amount_accepted() {
    let fx = Fixture::new();

    fx.client.mock_all_auths().set_escrow_amount_limits(&fx.admin, &100, &1000);

    let amount = 500;
    let fee_bps = 100;
    let shipping_window = 86400;

    let escrow_id = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount,
        &fee_bps,
        &shipping_window,
    );

    assert_eq!(escrow_id, 1);
}

#[test]
fn test_below_minimum_rejected() {
    let fx = Fixture::new();

    fx.client.mock_all_auths().set_escrow_amount_limits(&fx.admin, &100, &1000);

    let amount = 99; // Below min
    let fee_bps = 100;
    let shipping_window = 86400;

    let res = fx.client.mock_all_auths().try_create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount,
        &fee_bps,
        &shipping_window,
    );

    assert_eq!(res, Err(Ok(ContractError::InvalidAmount)));
    
    // Counter unchanged
    let stats = fx.client.get_stats();
    assert_eq!(stats.total_created, 0);
}

#[test]
fn test_above_maximum_rejected() {
    let fx = Fixture::new();

    fx.client.mock_all_auths().set_escrow_amount_limits(&fx.admin, &100, &1000);

    let amount = 1001; // Above max
    let fee_bps = 100;
    let shipping_window = 86400;

    let res = fx.client.mock_all_auths().try_create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount,
        &fee_bps,
        &shipping_window,
    );

    assert_eq!(res, Err(Ok(ContractError::AmountExceedsMaximum)));
}

#[test]
fn test_boundary_values() {
    let fx = Fixture::new();

    fx.client.mock_all_auths().set_escrow_amount_limits(&fx.admin, &100, &1000);

    let fee_bps = 100;
    let shipping_window = 86400;

    // Test exact minimum
    let id1 = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &100,
        &fee_bps,
        &shipping_window,
    );
    assert_eq!(id1, 1);

    // Test exact maximum
    let id2 = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &Address::generate(&fx.env), // Use different resolver to avoid any other potential conflicts
        &fx.token.address,
        &1000,
        &fee_bps,
        &shipping_window,
    );
    assert_eq!(id2, 2);
}

#[test]
fn test_invalid_admin_configuration() {
    let fx = Fixture::new();

    // min > max
    let res = fx.client.mock_all_auths().try_set_escrow_amount_limits(&fx.admin, &1000, &100);
    assert_eq!(res, Err(Ok(ContractError::InvalidAmount)));
    
    // min < hard limit
    let res2 = fx.client.mock_all_auths().try_set_escrow_amount_limits(&fx.admin, &0, &1000);
    assert_eq!(res2, Err(Ok(ContractError::InvalidAmount)));
}

#[test]
fn test_unauthorized_update() {
    let fx = Fixture::new();
    let unauthorized = Address::generate(&fx.env);

    let res = fx.client.try_set_escrow_amount_limits(&unauthorized, &100, &1000);
    assert!(res.is_err());
}
