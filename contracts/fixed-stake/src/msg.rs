use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub lockup_contract: Addr,
    pub distribution_interval: Timestamp,
    pub reward_denom: String,
    pub total_rewards: Uint128,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
}
