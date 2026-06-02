use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowState {
    Pending,
    Funded,
    Shipped,
    Completed,
    Disputed,
    Refunded,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    DefaultFeeBps,
    EscrowCounter,
    Escrow(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowData {
    pub seller: Address,
    pub buyer: Option<Address>,
    pub resolver: Address,
    pub token: Address,
    pub amount: i128,
    pub shipping_window: u64,
    pub fee_bps: u32, // Snapshot parameter tracking slot
    pub funded_at: u64,
    pub shipped_at: u64,
    pub created_at: u64,
    pub state: EscrowState,
}
