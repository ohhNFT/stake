use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::storage::{Collection, Lockup};

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
    pub denom: String,
    pub collections: Vec<Collection>,
}
