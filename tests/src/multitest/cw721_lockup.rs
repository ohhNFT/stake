#![cfg(test)]

use cosmwasm_std::{coins, Addr, Empty, StdError, Timestamp};
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

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
        cw721_lockup::contract::entry_points::execute,
        cw721_lockup::contract::entry_points::instantiate,
        cw721_lockup::contract::entry_points::query,
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

    // Set up Cw721Lockup contract
    let lockup_id = router.store_code(contract_lockup());
    let msg = cw721_lockup::contract::InstantiateMsg {
        lockup_interval: Some(Timestamp::from_seconds(3600)),
        collections: vec![cw721_addr.to_string()],
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

// Mint a CW721 NFT to an address
fn mint_cw721(router: &mut App, addr: Addr, token_id: &str) {
    let msg: Cw721ExecuteMsg<Empty, Empty> = Cw721ExecuteMsg::Mint {
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
    let msg: Cw721ExecuteMsg<Empty, Empty> = Cw721ExecuteMsg::SendNft {
        contract: recipient.to_string(),
        token_id: token_id.to_string(),
        msg: b"{}".to_vec().into(),
    };

    router
        .execute_contract(sender, Addr::unchecked(CW721), &msg, &[])
        .unwrap();
}

#[test]
fn proper_initialization() {
    setup_contracts();
}

#[test]
fn try_query_config() {
    let router = setup_contracts();
    let msg = cw721_lockup::contract::QueryMsg::Config {};
    let res: cw721_lockup::msg::ConfigResponse =
        router.wrap().query_wasm_smart(LOCKUP, &msg).unwrap();
    assert_eq!(res.admin, ADMIN);
    assert_eq!(res.lockup_interval, Timestamp::from_seconds(3600));
    assert_eq!(res.collections.len(), 1);
    assert_eq!(res.collections[0], Addr::unchecked(CW721));
}

#[test]
fn try_deposit_cw721() {
    let mut router = setup_contracts();

    let user = Addr::unchecked(USER);
    let contract = Addr::unchecked(LOCKUP);
    let token_id = "1";

    mint_cw721(&mut router, user.clone(), "1");
    send_cw721(&mut router, user.clone(), contract, token_id);

    let msg = cw721_lockup::contract::QueryMsg::LockupsByOwner {
        owner: user.to_string(),
    };

    let res: cw721_lockup::msg::LockupsReponse =
        router.wrap().query_wasm_smart(LOCKUP, &msg).unwrap();
    assert_eq!(res.lockups.len(), 1);
    assert_eq!(res.lockups[0].owner, user);
    assert_eq!(res.lockups[0].collection_address, Addr::unchecked(CW721));
    assert_eq!(res.lockups[0].token_id, token_id);
}

#[test]
fn try_withdraw_cw721() {
    let mut router = setup_contracts();

    let user = Addr::unchecked(USER);
    let admin = Addr::unchecked(ADMIN);
    let contract = Addr::unchecked(LOCKUP);
    let token_id = "1";

    mint_cw721(&mut router, user.clone(), "1");
    send_cw721(&mut router, user.clone(), contract.clone(), token_id);

    let msg = cw721_lockup::contract::QueryMsg::LockupsByOwner {
        owner: user.to_string(),
    };

    let res: cw721_lockup::msg::LockupsReponse =
        router.wrap().query_wasm_smart(LOCKUP, &msg).unwrap();
    assert_eq!(res.lockups.len(), 1);

    let msg = cw721_lockup::contract::ExecMsg::Withdraw {
        collection_address: CW721.to_string(),
        token_id: token_id.to_string(),
    };

    let err = router
        .execute_contract(user.clone(), contract.clone(), &msg, &[])
        .unwrap_err();

    assert_eq!(
        err.downcast::<StdError>().unwrap(),
        StdError::generic_err("Lockup period has not passed")
    );

    add_block_time(&mut router, 3700);

    let err = router
        .execute_contract(admin.clone(), contract.clone(), &msg, &[])
        .unwrap_err();

    assert_eq!(
        err.downcast::<StdError>().unwrap(),
        StdError::generic_err("Sender is not the owner of the NFT")
    );

    router
        .execute_contract(user.clone(), contract, &msg, &[])
        .unwrap();

    let msg = cw721_lockup::contract::QueryMsg::LockupsByOwner {
        owner: user.to_string(),
    };

    let res: cw721_lockup::msg::LockupsReponse =
        router.wrap().query_wasm_smart(LOCKUP, &msg).unwrap();
    assert_eq!(res.lockups.len(), 0);
}
