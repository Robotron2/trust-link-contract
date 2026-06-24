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
    fee_collector: Address,
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
        // Initialize with 500 bps arbitration fee
        client.initialize(&admin, &fee_collector, &500);

        // Set protocol fee to 100 bps (1%)
        client.mock_all_auths().set_protocol_fee(&admin, &100);

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
            fee_collector,
            seller,
            buyer,
            resolver,
        }
    }
}

#[test]
fn test_no_fees() {
    let fx = Fixture::new();
    let accumulated = fx.client.get_accumulated_fees(&fx.token.address);
    assert_eq!(accumulated, 0);
}

#[test]
fn test_single_fee_event_confirm_delivery() {
    let fx = Fixture::new();
    let amount = 100_000;
    fx.token_admin.mint(&fx.buyer, &amount);

    let escrow_id = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount,
        &100, // Escrow fee (deducted from seller payout) is 100 bps = 1% = 1_000
        &0,
    );
    fx.client.mock_all_auths().fund_escrow(&fx.buyer, &escrow_id);
    fx.client.mock_all_auths().mark_shipped(&fx.seller, &escrow_id, &"TRK123".into());
    fx.client.mock_all_auths().record_delivery(&fx.buyer, &escrow_id);
    
    fx.env.ledger().set_timestamp(fx.env.ledger().timestamp() + 86400 * 3 + 1);

    // confirm_delivery pays out. Protocol fee is 100 bps (1%) of 100_000 = 1_000
    fx.client.mock_all_auths().confirm_delivery(&fx.buyer, &escrow_id);

    let accumulated = fx.client.get_accumulated_fees(&fx.token.address);
    assert_eq!(accumulated, 1000);
}

#[test]
fn test_multiple_fee_events() {
    let fx = Fixture::new();
    let amount1 = 100_000;
    let amount2 = 200_000;
    fx.token_admin.mint(&fx.buyer, &(amount1 + amount2));

    // Escrow 1
    let id1 = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount1,
        &100,
        &0,
    );
    fx.client.mock_all_auths().fund_escrow(&fx.buyer, &id1);
    fx.client.mock_all_auths().mark_shipped(&fx.seller, &id1, &"TRK1".into());
    fx.client.mock_all_auths().record_delivery(&fx.buyer, &id1);

    // Escrow 2
    let id2 = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount2,
        &100,
        &0,
    );
    fx.client.mock_all_auths().fund_escrow(&fx.buyer, &id2);
    fx.client.mock_all_auths().mark_shipped(&fx.seller, &id2, &"TRK2".into());
    fx.client.mock_all_auths().record_delivery(&fx.buyer, &id2);

    fx.env.ledger().set_timestamp(fx.env.ledger().timestamp() + 86400 * 3 + 1);

    fx.client.mock_all_auths().confirm_delivery(&fx.buyer, &id1);
    fx.client.mock_all_auths().confirm_delivery(&fx.buyer, &id2);

    let accumulated = fx.client.get_accumulated_fees(&fx.token.address);
    // fee1 = 1% of 100_000 = 1000
    // fee2 = 1% of 200_000 = 2000
    assert_eq!(accumulated, 3000);
}

#[test]
fn test_multiple_tokens() {
    let fx = Fixture::new();
    
    // Create token B
    let token_admin_b = Address::generate(&fx.env);
    let token_addr_b = fx.env.register_stellar_asset_contract(token_admin_b.clone());
    let token_b = token::StellarAssetClient::new(&fx.env, &token_addr_b);
    let token_admin_client_b = token::StellarAssetClient::new(&fx.env, &token_addr_b);

    fx.token_admin.mint(&fx.buyer, &100_000);
    token_admin_client_b.mint(&fx.buyer, &500_000);

    // Escrow with Token A
    let id_a = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &100_000,
        &100,
        &0,
    );
    fx.client.mock_all_auths().fund_escrow(&fx.buyer, &id_a);
    fx.client.mock_all_auths().mark_shipped(&fx.seller, &id_a, &"TRK".into());
    fx.client.mock_all_auths().record_delivery(&fx.buyer, &id_a);

    // Escrow with Token B
    let id_b = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &token_b.address,
        &500_000,
        &100,
        &0,
    );
    fx.client.mock_all_auths().fund_escrow(&fx.buyer, &id_b);
    fx.client.mock_all_auths().mark_shipped(&fx.seller, &id_b, &"TRK".into());
    fx.client.mock_all_auths().record_delivery(&fx.buyer, &id_b);

    fx.env.ledger().set_timestamp(fx.env.ledger().timestamp() + 86400 * 3 + 1);

    fx.client.mock_all_auths().confirm_delivery(&fx.buyer, &id_a);
    fx.client.mock_all_auths().confirm_delivery(&fx.buyer, &id_b);

    let acc_a = fx.client.get_accumulated_fees(&fx.token.address);
    let acc_b = fx.client.get_accumulated_fees(&token_b.address);

    assert_eq!(acc_a, 1000);
    assert_eq!(acc_b, 5000);
}

#[test]
fn test_withdrawal_updates() {
    let fx = Fixture::new();
    let amount = 100_000;
    fx.token_admin.mint(&fx.buyer, &amount);

    let id = fx.client.mock_all_auths().create_escrow(
        &fx.seller,
        &Some(fx.buyer.clone()),
        &fx.resolver,
        &fx.token.address,
        &amount,
        &100,
        &0,
    );
    fx.client.mock_all_auths().fund_escrow(&fx.buyer, &id);
    fx.client.mock_all_auths().mark_shipped(&fx.seller, &id, &"TRK".into());
    fx.client.mock_all_auths().record_delivery(&fx.buyer, &id);
    
    fx.env.ledger().set_timestamp(fx.env.ledger().timestamp() + 86400 * 3 + 1);
    fx.client.mock_all_auths().confirm_delivery(&fx.buyer, &id);

    let acc_before = fx.client.get_accumulated_fees(&fx.token.address);
    assert_eq!(acc_before, 1000);

    // Withdraw 400
    fx.client.mock_all_auths().withdraw_fees(&fx.admin, &fx.token.address, &fx.fee_collector, &400);

    let acc_after = fx.client.get_accumulated_fees(&fx.token.address);
    assert_eq!(acc_after, 600); // 1000 - 400 = 600
}
