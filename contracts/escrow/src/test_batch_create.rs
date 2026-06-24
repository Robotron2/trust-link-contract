#![cfg(test)]
extern crate std;

use crate::{ContractError, Escrow, EscrowClient, EscrowCreationRequest};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

struct Fixture {
    env: Env,
    client: EscrowClient<'static>,
    token: token::StellarAssetClient<'static>,
    token_admin: token::StellarAssetClient<'static>,
    seller: Address,
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

        Self {
            env,
            client,
            token,
            token_admin: token_admin_client,
            seller,
        }
    }
}

#[test]
fn test_batch_create_multiple_escrows() {
    let fx = Fixture::new();

    let mut requests = Vec::new(&fx.env);
    
    // Create 3 requests
    for _ in 0..3 {
        let buyer = Address::generate(&fx.env);
        let resolver = Address::generate(&fx.env);
        requests.push_back(EscrowCreationRequest {
            buyer: Some(buyer),
            resolver,
            token: fx.token.address.clone(),
            amount: 100_000_000,
            fee_bps: 100,
            shipping_window: 86400,
        });
    }

    let stats_before = fx.client.get_stats();
    assert_eq!(stats_before.total_created, 0);

    let created_ids = fx.client.mock_all_auths().batch_create_escrow(&fx.seller, &requests);
    assert_eq!(created_ids.len(), 3);
    assert_eq!(created_ids.get(0).unwrap(), 1);
    assert_eq!(created_ids.get(1).unwrap(), 2);
    assert_eq!(created_ids.get(2).unwrap(), 3);

    let stats_after = fx.client.get_stats();
    assert_eq!(stats_after.total_created, 3);

    // Verify escrows exist
    for id in created_ids.into_iter() {
        let escrow = fx.client.get_escrow(&id);
        assert_eq!(escrow.seller, fx.seller);
        assert_eq!(escrow.amount, 100_000_000);
    }
}

#[test]
fn test_batch_empty() {
    let fx = Fixture::new();
    let requests = Vec::new(&fx.env);

    let created_ids = fx.client.mock_all_auths().batch_create_escrow(&fx.seller, &requests);
    assert_eq!(created_ids.len(), 0);

    let stats = fx.client.get_stats();
    assert_eq!(stats.total_created, 0);
}

#[test]
fn test_batch_invalid_item_rollback() {
    let fx = Fixture::new();

    let mut requests = Vec::new(&fx.env);
    
    // Valid
    requests.push_back(EscrowCreationRequest {
        buyer: Some(Address::generate(&fx.env)),
        resolver: Address::generate(&fx.env),
        token: fx.token.address.clone(),
        amount: 100_000_000,
        fee_bps: 100,
        shipping_window: 86400,
    });
    
    // Invalid (negative amount)
    requests.push_back(EscrowCreationRequest {
        buyer: Some(Address::generate(&fx.env)),
        resolver: Address::generate(&fx.env),
        token: fx.token.address.clone(),
        amount: -10,
        fee_bps: 100,
        shipping_window: 86400,
    });

    let res = fx.client.mock_all_auths().try_batch_create_escrow(&fx.seller, &requests);
    assert_eq!(res, Err(Ok(ContractError::InvalidAmount)));

    let stats = fx.client.get_stats();
    assert_eq!(stats.total_created, 0);

    // Attempt to retrieve escrow 1 should fail
    let res = fx.client.try_get_escrow(&1);
    assert_eq!(res, Err(Ok(ContractError::EscrowNotFound)));
}

#[test]
fn test_batch_unauthorized() {
    let fx = Fixture::new();
    let unauthorized = Address::generate(&fx.env);

    let mut requests = Vec::new(&fx.env);
    requests.push_back(EscrowCreationRequest {
        buyer: Some(Address::generate(&fx.env)),
        resolver: Address::generate(&fx.env),
        token: fx.token.address.clone(),
        amount: 100_000_000,
        fee_bps: 100,
        shipping_window: 86400,
    });

    let res = fx.client.try_batch_create_escrow(&unauthorized, &requests);
    assert!(res.is_err());
}
