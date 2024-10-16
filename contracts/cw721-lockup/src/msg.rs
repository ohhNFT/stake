use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};

use crate::storage::Lockup;

#[cw_serde]
pub struct CountResponse {
    pub count: u128,
}

#[cw_serde]
pub struct LockupsReponse {
    pub lockups: Vec<Lockup>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub lockup_interval: Timestamp,
    pub collections: Vec<Addr>,
}
