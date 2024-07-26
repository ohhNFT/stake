use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};

#[cw_serde]
pub struct Stake {
    pub last_claim: Timestamp,
}

impl Stake {
    pub fn new(last_claim: Timestamp) -> Self {
        Self { last_claim }
    }
}

#[cw_serde]
pub struct LockupInfo {
    pub owner: Option<Addr>,
    pub amount: Uint128,
    pub locked_since: Timestamp,
    pub locked_until: Timestamp,
}
