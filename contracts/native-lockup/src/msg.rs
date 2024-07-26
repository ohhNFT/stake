use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};

#[cw_serde]
pub struct CountResponse {
    pub count: u128,
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub lockup_interval: Timestamp,
    pub token: String,
}

#[cw_serde]
pub struct ContractTypeResponse {
    pub contract_type: String,
}
