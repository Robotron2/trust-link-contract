#![cfg(test)]
extern crate std;

use crate::{ContractError, Escrow, EscrowClient, EscrowState};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Symbol,
};

struct Fixture {
    env: Env,
    client: EscrowClient<'static>,
    token: token::StellarAssetClient<'static>,
    token_admin: token::StellarAssetClient<'static>,
    seller: Address,
    buyer: Address,
    resolver: Address,
    escrow_id: u64,
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

        let amount = 100_000_000;
        let fee_bps = 100;
        let shipping_window = 86400;

        // Give buyer funds
        token_admin_client.mint(&buyer, &amount);

        let escrow_id = client.create_escrow(
            &seller,
            &Some(buyer.clone()),
            &resolver,
            &token_addr,
            &amount,
            &fee_bps,
            &shipping_window,
        );

        // Fund escrow
        client.mock_all_auths().fund_escrow(&buyer, &escrow_id);

        Self {
            env,
            client,
            token,
            token_admin: token_admin_client,
            seller,
            buyer,
            resolver,
            escrow_id,
        }
    }
}

#[test]
fn test_seller_approves_refund() {
    let fx = Fixture::new();

    // Buyer requests refund
    fx.client.mock_all_auths().request_refund(&fx.buyer, &fx.escrow_id);

    let escrow = fx.client.get_escrow(&fx.escrow_id);
    assert_eq!(escrow.state, EscrowState::RefundRequested);

    let contract_addr = fx.client.address.clone();
    let contract_balance_before = fx.token.balance(&contract_addr);
    let buyer_balance_before = fx.token.balance(&fx.buyer);

    // Seller approves refund
    fx.client.mock_all_auths().approve_refund(&fx.seller, &fx.escrow_id);

    let escrow_after = fx.client.get_escrow(&fx.escrow_id);
    assert_eq!(escrow_after.state, EscrowState::Refunded);

    let contract_balance_after = fx.token.balance(&contract_addr);
    let buyer_balance_after = fx.token.balance(&fx.buyer);

    assert_eq!(contract_balance_after, contract_balance_before - escrow.amount);
    assert_eq!(buyer_balance_after, buyer_balance_before + escrow.amount);
    assert_eq!(contract_balance_after, 0); // Contract is empty since we minted exact amount
}

#[test]
fn test_invalid_state_approve_refund() {
    let fx = Fixture::new();

    // Trying to approve refund when state is Funded should fail
    let res = fx.client.mock_all_auths().try_approve_refund(&fx.seller, &fx.escrow_id);
    assert_eq!(res, Err(Ok(ContractError::InvalidState)));
}

#[test]
fn test_unauthorized_seller_approve_refund() {
    let fx = Fixture::new();

    fx.client.mock_all_auths().request_refund(&fx.buyer, &fx.escrow_id);

    // Trying to approve refund with buyer instead of seller
    let res = fx.client.mock_all_auths().try_approve_refund(&fx.buyer, &fx.escrow_id);
    assert_eq!(res, Err(Ok(ContractError::NotAuthorized)));
}

#[test]
fn test_double_approval() {
    let fx = Fixture::new();

    fx.client.mock_all_auths().request_refund(&fx.buyer, &fx.escrow_id);
    fx.client.mock_all_auths().approve_refund(&fx.seller, &fx.escrow_id);

    // Second approval should fail
    let res = fx.client.mock_all_auths().try_approve_refund(&fx.seller, &fx.escrow_id);
    assert_eq!(res, Err(Ok(ContractError::InvalidState)));
}
