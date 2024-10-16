#![cfg(test)]

use cosmwasm_std::{coin, coins, Addr, BankMsg, Empty, StdError, Timestamp, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub fn contract_native_lockup() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        native_lockup::contract::entry_points::execute,
        native_lockup::contract::entry_points::instantiate,
        native_lockup::contract::entry_points::query,
    );
    Box::new(contract)
}

pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub fn contract_cw721_lockup() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_lockup::contract::entry_points::execute,
        cw721_lockup::contract::entry_points::instantiate,
        cw721_lockup::contract::entry_points::query,
    );
    Box::new(contract)
}

pub fn contract_stake() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::entry_points::execute,
        crate::contract::entry_points::instantiate,
        crate::contract::entry_points::query,
    );
    Box::new(contract)
}

const NATIVE_LOCKUP: &str = "contract0";
const CW721: &str = "contract0";
const CW721_LOCKUP: &str = "contract1";
const NATIVE_STAKE: &str = "contract1";
const CW721_STAKE: &str = "contract2";

const ADMIN: &str = "admin";
const USER: &str = "user";

fn setup_native_contracts() -> App {
    let admin = Addr::unchecked(ADMIN);

    let init_funds = coins(200, "ustars");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &admin, init_funds)
            .unwrap();
    });

    // Set up NativeLockup contract
    let lockup_id = router.store_code(contract_native_lockup());
    let msg = native_lockup::contract::InstantiateMsg {
        lockup_interval: Some(Timestamp::from_seconds(3600)),
        token: "ustars".to_string(),
    };

    router
        .instantiate_contract(lockup_id, admin.clone(), &msg, &[], "NATIVE_LOCKUP", None)
        .unwrap();

    // Set up FixedStake contract
    let stake_id = router.store_code(contract_stake());
    let msg = crate::contract::InstantiateMsg {
        lockup_contract: NATIVE_LOCKUP.to_string(),
        distribution_interval: Timestamp::from_seconds(3600),
        reward_denom: "ustars".to_string(),
        total_rewards: Uint128::from(100u128),
        start_time: Timestamp::from_seconds(1),
        end_time: Timestamp::from_seconds(36001),
    };

    router
        .instantiate_contract(stake_id, admin.clone(), &msg, &[], "LOCKUP", None)
        .unwrap();

    // Admin send 100 ustars to contract
    router
        .execute(
            admin.clone(),
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                to_address: NATIVE_STAKE.to_string(),
                amount: vec![coin(100, "ustars")],
            }),
        )
        .unwrap();

    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(1);
    router.set_block(block);

    router
}

fn setup_cw721_contracts() -> App {
    let admin = Addr::unchecked(ADMIN);

    let init_funds = coins(100, "ustars");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &admin, init_funds)
            .unwrap();
    });

    // Set up Cw721Lockup contract
    let cw721_id = router.store_code(contract_cw721());
    let msg = cw721_base::msg::InstantiateMsg {
        name: String::from("Bad Kids"),
        symbol: String::from("BAD"),
        minter: admin.to_string(),
    };
    let cw721_addr = router
        .instantiate_contract(cw721_id, admin.clone(), &msg, &[], "CW721", None)
        .unwrap();
    let lockup_id = router.store_code(contract_cw721_lockup());
    let msg = cw721_lockup::contract::InstantiateMsg {
        lockup_interval: Some(Timestamp::from_seconds(3600)),
        collections: vec![cw721_addr.to_string()],
    };

    router
        .instantiate_contract(lockup_id, admin.clone(), &msg, &[], "LOCKUP", None)
        .unwrap();

    // Set up FixedStake contract
    let stake_id = router.store_code(contract_stake());
    let msg = crate::contract::InstantiateMsg {
        lockup_contract: CW721_LOCKUP.to_string(),
        distribution_interval: Timestamp::from_seconds(3600),
        reward_denom: "ustars".to_string(),
        total_rewards: Uint128::from(100u128),
        start_time: Timestamp::from_seconds(1),
        end_time: Timestamp::from_seconds(36001),
    };

    router
        .instantiate_contract(stake_id, admin.clone(), &msg, &[], "STAKE", None)
        .unwrap();

    // Admin send 100 ustars to contract
    router
        .execute(
            admin.clone(),
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                to_address: CW721_STAKE.to_string(),
                amount: vec![coin(100, "ustars")],
            }),
        )
        .unwrap();

    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(1);
    router.set_block(block);

    router
}

// Update block time
fn add_block_time(router: &mut App, seconds: u64) {
    let mut block = router.block_info();
    block.time = block.time.plus_seconds(seconds);
    router.set_block(block);
}

#[test]
fn proper_native_initialization() {
    setup_native_contracts();
    println!("{:?}", Timestamp::from_seconds(3600).to_string());
    println!("{:?}", Timestamp::from_seconds(1721962842).to_string());
    println!(
        "{:?}",
        Timestamp::from_seconds(1721962842).plus_days(7).to_string()
    )
}

#[test]
fn proper_cw721_initialization() {
    setup_cw721_contracts();
}

// Mint a CW721 NFT to an address
fn mint_cw721(router: &mut App, addr: Addr, token_id: &str) {
    let msg: cw721_base::ExecuteMsg<Empty, Empty> = cw721_base::ExecuteMsg::Mint {
        token_id: token_id.to_string(),
        owner: addr.to_string(),
        token_uri: None,
        extension: Empty {},
    };

    router
        .execute_contract(Addr::unchecked(ADMIN), Addr::unchecked(CW721), &msg, &[])
        .unwrap();
}

// Send a CW721 NFT to a contract
fn send_cw721(router: &mut App, sender: Addr, recipient: Addr, token_id: &str) {
    let msg: cw721_base::ExecuteMsg<Empty, Empty> = cw721_base::ExecuteMsg::SendNft {
        contract: recipient.to_string(),
        token_id: token_id.to_string(),
        msg: b"{}".to_vec().into(),
    };

    router
        .execute_contract(sender, Addr::unchecked(CW721), &msg, &[])
        .unwrap();
}

#[test]
fn native_deposit_and_claim() {
    let mut router = setup_native_contracts();
    let admin = Addr::unchecked(ADMIN);
    let user = Addr::unchecked(USER);

    let deposit_amount = coins(100, "ustars");

    // Admin sends 100 ustars to user
    router
        .execute(
            admin.clone(),
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send {
                to_address: user.to_string(),
                amount: vec![coin(100, "ustars")],
            }),
        )
        .unwrap();

    // User deposits 100 ustars
    let msg = native_lockup::contract::ExecMsg::Deposit {};
    router
        .execute_contract(
            user.clone(),
            Addr::unchecked(NATIVE_LOCKUP),
            &msg,
            &deposit_amount,
        )
        .unwrap();

    // User claims rewards before they are available
    let msg = crate::contract::ExecMsg::ClaimRewards {
        of: (USER.to_string(), String::from("")),
    };
    let err = router
        .execute_contract(user.clone(), Addr::unchecked(NATIVE_STAKE), &msg, &[])
        .unwrap_err();

    assert_eq!(
        err.downcast::<StdError>().unwrap(),
        StdError::generic_err("Reward distribution period has not started")
    );

    // Time advances by 3700 seconds
    add_block_time(&mut router, 3700);

    // User claims rewards
    router
        .execute_contract(user.clone(), Addr::unchecked(NATIVE_STAKE), &msg, &[])
        .unwrap();

    // Verify that the user has 10 stars
    let balance = router.wrap().query_balance(USER, "ustars").unwrap();
    assert_eq!(balance, coin(10u128, "ustars"));
}

#[test]
fn cw721_deposit_and_claim() {
    let mut router = setup_cw721_contracts();
    let user = Addr::unchecked(USER);

    let token_id = "1";

    mint_cw721(&mut router, user.clone(), "1");
    send_cw721(
        &mut router,
        user.clone(),
        Addr::unchecked(CW721_LOCKUP),
        token_id,
    );

    // User claims rewards before they are available
    let msg = crate::contract::ExecMsg::ClaimRewards {
        of: (CW721.to_string(), token_id.to_string()),
    };
    let err = router
        .execute_contract(user.clone(), Addr::unchecked(CW721_STAKE), &msg, &[])
        .unwrap_err();

    assert_eq!(
        err.downcast::<StdError>().unwrap(),
        StdError::generic_err("Reward distribution period has not started")
    );

    // Time advances by 3700 seconds
    add_block_time(&mut router, 3700);

    // User claims rewards
    router
        .execute_contract(user.clone(), Addr::unchecked(CW721_STAKE), &msg, &[])
        .unwrap();

    // Verify that the user has 10 stars
    let balance = router.wrap().query_balance(USER, "ustars").unwrap();
    assert_eq!(balance, coin(10u128, "ustars"));
}
