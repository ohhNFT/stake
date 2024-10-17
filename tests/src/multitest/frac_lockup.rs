#![cfg(test)]

use cosmwasm_std::{coins, Addr, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use frac_lockup::storage::CollectionInput;

pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub fn contract_lockup() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        frac_lockup::contract::entry_points::execute,
        frac_lockup::contract::entry_points::instantiate,
        frac_lockup::contract::entry_points::query,
    );
    Box::new(contract)
}

const CW721: &str = "contract0";
const LOCKUP: &str = "contract1";

const ADMIN: &str = "admin";
const USER: &str = "user";

// Initial contract setup
fn setup_contracts() -> App {
    let admin = Addr::unchecked(ADMIN);

    let init_funds = coins(2000, "ustars");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &admin, init_funds)
            .unwrap();
    });

    // Set up CW721 contract
    let cw721_id = router.store_code(contract_cw721());
    let msg = cw721_base::msg::InstantiateMsg {
        name: String::from("Bad Kids"),
        symbol: String::from("BAD"),
        minter: admin.to_string(),
    };

    let cw721_addr = router
        .instantiate_contract(cw721_id, admin.clone(), &msg, &[], "CW721", None)
        .unwrap();

    // Set up FracLockup contract
    let lockup_id = router.store_code(contract_lockup());
    let msg = frac_lockup::contract::InstantiateMsg {
        collections: vec![CollectionInput {
            address: cw721_addr.to_string(),
            tokens: 1000000u128,
        }],
        denom: format!("factory/{}/{}", admin.to_string(), "bad-kids"),
    };

    router
        .instantiate_contract(lockup_id, admin.clone(), &msg, &[], "LOCKUP", None)
        .unwrap();

    router
}

#[test]
fn proper_initialization() {
    setup_contracts();
}
