#![cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::{Escrow, EscrowClient};

#[test]
fn test_fee_change_does_not_affect_funded_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, Escrow);
    let client = EscrowClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);
    let resolver = Address::generate(&env);
    let token = Address::generate(&env);

    // 1. Initialize configuration profile with a 1% base fee fee_bps = 100
    client.initialize(&admin, &100);

    // 2. Create and fund escrow agreement under the 1% terms
    let escrow_amount = 1_000_000_i128;
    let escrow_id = client.create_escrow(&seller, &resolver, &token, &escrow_amount, &604800);
    client.fund_escrow(&escrow_id, &buyer);

    // 3. Admin alters global fee configurations upward to 3% fee_bps = 300 mid-flight
    client.set_fee(&admin, &300);

    // 4. Finalize trade processing
    let net_payout = client.confirm_delivery(&escrow_id);

    // Expected behavior: Payout calculations must use the snapshotted 1% fee rate, not the updated 3% rate.
    // 1% of 1,000,000 = 10,000 fee. Net payout should be 990,000.
    assert_eq!(net_payout, 990_000);

    // Assert that the escrow instance holds its immutable 1% parameters securely
    let escrow_state = client.get_escrow(&escrow_id);
    assert_eq!(escrow_state.fee_bps, 100);
}
