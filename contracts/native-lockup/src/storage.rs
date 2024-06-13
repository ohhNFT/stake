use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Timestamp, Uint128};

#[cw_serde]
pub struct Lockup {
    pub amount: Uint128,
    pub locked_until: Timestamp,
}

impl Lockup {
    pub fn new(amount: Uint128, locked_until: Timestamp) -> Self {
        Self {
            amount,
            locked_until,
        }
    }
}
