use cosmwasm_schema::write_api;
use sg721_lockup::contract::{ContractExecMsg, ContractQueryMsg, InstantiateMsg};

#[cfg(not(tarpaulin_include))]
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ContractExecMsg,
        query: ContractQueryMsg,
    }
}
