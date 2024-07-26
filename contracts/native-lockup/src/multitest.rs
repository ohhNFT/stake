#![cfg(test)]

use cosmwasm_std::{coin, coins, Addr, BankMsg, Empty, StdError, Timestamp, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub fn contract_lockup() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::entry_points::execute,
        crate::contract::entry_points::instantiate,
        crate::contract::entry_points::query,
    );
    Box::new(contract)
}

const LOCKUP: &str = "contract0";

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

    // Set up Cw721Lockup contract
    let lockup_id = router.store_code(contract_lockup());
    let msg = crate::contract::InstantiateMsg {
        lockup_interval: Timestamp::from_seconds(3600),
        token: "ustars".to_string(),
    };

    router
        .instantiate_contract(lockup_id, admin.clone(), &msg, &[], "LOCKUP", None)
        .unwrap();

    router
}

// Update block time
fn add_block_time(router: &mut App, seconds: u64) {
    let mut block = router.block_info();
    block.time = block.time.plus_seconds(seconds);
    router.set_block(block);
}

#[test]
fn proper_initialization() {
    setup_contracts();
}

#[test]
fn try_query_config() {
    let router = setup_contracts();
    let msg = crate::contract::QueryMsg::Config {};
    let res: crate::msg::ConfigResponse = router.wrap().query_wasm_smart(LOCKUP, &msg).unwrap();
    assert_eq!(res.admin, ADMIN);
    assert_eq!(res.lockup_interval, Timestamp::from_seconds(3600));
    assert_eq!(res.token, "ustars");
}

#[test]
fn try_deposit() {
    let mut router = setup_contracts();
    let admin = Addr::unchecked(ADMIN);
    let user = Addr::unchecked(USER);

    let deposit_amount = coins(500, "ustars");

    // Admin sends 1000 ustars to user
    router
        .execute(
            admin.clone(),
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                to_address: user.to_string(),
                amount: vec![coin(1000, "ustars")],
            }),
        )
        .unwrap();

    // User attempts to deposit 0 ustars
    let msg = crate::contract::ExecMsg::Deposit {};
    let err = router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &[])
        .unwrap_err();
    assert_eq!(
        err.downcast::<StdError>().unwrap(),
        StdError::generic_err("No funds sent")
    );

    // User deposits 500 ustars
    let msg = crate::contract::ExecMsg::Deposit {};
    router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &deposit_amount)
        .unwrap();

    // Query the lockup
    let query_msg = crate::contract::QueryMsg::Lockup {
        address: user.to_string(),
    };
    let res: crate::storage::Lockup = router.wrap().query_wasm_smart(LOCKUP, &query_msg).unwrap();
    assert_eq!(res.amount, deposit_amount[0].amount);

    // Deposit 500 more ustars
    router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &deposit_amount)
        .unwrap();

    // Query the lockup
    let res: crate::storage::Lockup = router.wrap().query_wasm_smart(LOCKUP, &query_msg).unwrap();
    assert_eq!(res.amount.u128(), deposit_amount[0].amount.u128() * 2);
}

#[test]
fn try_withdraw() {
    let mut router = setup_contracts();
    let admin = Addr::unchecked(ADMIN);
    let user = Addr::unchecked(USER);

    let deposit_amount = coins(1000, "ustars");
    let withdraw_amount = Uint128::from(500u128);

    // Admin sends 1000 ustars to user
    router
        .execute(
            admin.clone(),
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                to_address: user.to_string(),
                amount: vec![coin(1000, "ustars")],
            }),
        )
        .unwrap();

    // User deposits 1000 ustars
    let msg = crate::contract::ExecMsg::Deposit {};
    router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &deposit_amount)
        .unwrap();

    // Query the lockup
    let query_msg = crate::contract::QueryMsg::Lockup {
        address: user.to_string(),
    };
    let res: crate::storage::Lockup = router.wrap().query_wasm_smart(LOCKUP, &query_msg).unwrap();
    assert_eq!(res.amount, deposit_amount[0].amount);

    // Withdraw before lockup period has passed
    let msg = crate::contract::ExecMsg::Withdraw {
        amount: Some(withdraw_amount),
    };
    let err = router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &[])
        .unwrap_err();
    assert_eq!(
        err.downcast::<StdError>().unwrap(),
        StdError::generic_err("Lockup period has not passed")
    );

    // Update block time to pass lockup period
    add_block_time(&mut router, 3700);

    // Withdraw after lockup period has passed
    router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &[])
        .unwrap();

    // Query the lockup
    let res: crate::storage::Lockup = router.wrap().query_wasm_smart(LOCKUP, &query_msg).unwrap();
    assert_eq!(
        res.amount.u128(),
        deposit_amount[0].amount.u128() - withdraw_amount.u128()
    );

    // Withdraw the remaining amount
    let msg = crate::contract::ExecMsg::Withdraw { amount: None };
    router
        .execute_contract(user.clone(), Addr::unchecked(LOCKUP), &msg, &[])
        .unwrap();

    // Verify that querying the lockup returns an error
    // let err = router
    //     .wrap()
    //     .query_wasm_smart::<crate::storage::Lockup>(LOCKUP, &query_msg)
    //     .unwrap_err();
    // assert_eq!(
    //     err,
    //     StdError::generic_err("Querier contract error: type: native_lockup::storage::Lockup; key: [00, 06, 6C, 6F, 63, 6B, 75, 70, 75, 73, 65, 72] not found")
    // );
}
