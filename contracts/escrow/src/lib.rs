import { Env, Address, BytesN, Symbol, log };
use crate::types::{EscrowData, EscrowState, DataKey, Error};

// Realize structural definitions for the mock pipeline execution context
#[soroban_sdk::contract]
pub struct Escrow;

#[soroban_sdk::contractimpl]
impl Escrow {
    pub fn initialize(env: Env, admin: Address, default_fee_bps: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract is already initialized.");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::DefaultFeeBps, &default_fee_bps);
        env.storage().instance().set(&DataKey::EscrowCounter, &1_u32);
    }

    pub fn set_fee(env: Env, admin: Address, new_fee_bps: u32) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != current_admin {
            panic!("Unauthorized access boundaries.");
        }
        env.storage().instance().set(&DataKey::DefaultFeeBps, &new_fee_bps);
    }

    pub fn create_escrow(
        env: Env,
        seller: Address,
        resolver: Address,
        token: Address,
        amount: i128,
        shipping_window: u64
    ) -> u32 {
        seller.require_auth();

        let mut counter: u32 = env.storage().instance().get(&DataKey::EscrowCounter).unwrap_or(1);
        let global_fee_bps: u32 = env.storage().instance().get(&DataKey::DefaultFeeBps).unwrap_or(0);

        // FIX: Explicitly snapshot the global contract fee parameters at the precise moment of creation
        let escrow_data = EscrowData {
            seller: seller.clone(),
            buyer: None,
            resolver,
            token,
            amount,
            shipping_window,
            fee_bps: global_fee_bps, // Structural fix applied here
            funded_at: 0,
            shipped_at: 0,
            created_at: env.ledger().timestamp(),
            state: EscrowState::Pending,
        };

        env.storage().persistent().set(&DataKey::Escrow(counter), &escrow_data);
        env.storage().instance().set(&DataKey::EscrowCounter, &(counter + 1));

        counter
    }

    pub fn fund_escrow(env: Env, escrow_id: u32, buyer: Address) {
        buyer.require_auth();
        let mut escrow = Self::get_escrow(env.clone(), escrow_id);
        if !matches!(escrow.state, EscrowState::Pending) {
            panic!("Escrow is not pending.");
        }

        escrow.buyer = Some(buyer);
        escrow.state = EscrowState::Funded;
        escrow.funded_at = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);
    }

    pub fn confirm_delivery(env: Env, escrow_id: u32) -> i128 {
        let mut escrow = Self::get_escrow(env.clone(), escrow_id);
        let buyer = escrow.buyer.clone().expect("Escrow has no designated funding buyer profile context.");
        buyer.require_auth();

        // Enforce parsing calculation based strictly on the immutable snapshotted instance fee
        let fee_payout = (escrow.amount * escrow.fee_bps as i128) / 10000;
        let net_vendor_payout = escrow.amount - fee_payout;

        escrow.state = EscrowState::Completed;
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &escrow);

        net_vendor_payout
    }

    pub fn get_escrow(env: Env, escrow_id: u32) -> EscrowData {
        env.storage().persistent().get(&DataKey::Escrow(escrow_id)).expect("Escrow instance data payload not found.")
    }
}
