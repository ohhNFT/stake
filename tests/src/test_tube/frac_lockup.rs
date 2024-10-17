#![cfg(test)]

use std::rc::Rc;

use cosmwasm_std::{coin, coins, Uint128};

use cw_orch::prelude::*;
use cw_orch_osmosis_test_tube::OsmosisTestTube;
use frac_lockup::{
    msg::{ConfigResponse, LockupsReponse},
    storage::CollectionInput,
};
use osmosis_test_tube::osmosis_std::types::{
    cosmwasm::wasm::v1::{MsgInstantiateContract, MsgInstantiateContractResponse},
    osmosis::tokenfactory::v1beta1::{
        MsgChangeAdmin, MsgChangeAdminResponse, MsgCreateDenom, MsgCreateDenomResponse,
    },
};
use osmosis_test_tube::{Account, SigningAccount};
use prost::Message;
use prost_types::Any;

use crate::interface::cw721_base::{
    Cw721Base, Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg,
};
use crate::interface::frac_lockup::{
    ExecuteMsg as FracExecuteMsg, FracLockup, InstantiateMsg as FracInstantiateMsg,
    QueryMsg as FracQueryMsg,
};

pub const SUBDENOM: &str = "bad-kids";

#[derive(Clone)]
struct TestState {
    pub admin: Rc<SigningAccount>,
    pub chain: OsmosisTestTube,
    pub cw721_base: Cw721Base<OsmosisTestTube>,
    pub frac_lockup: FracLockup<OsmosisTestTube>,
    pub denom: String,
}

fn setup_contracts() -> cw_orch::anyhow::Result<TestState> {
    let _ = env_logger::try_init();
    let mut chain = OsmosisTestTube::new(coins(1_000_000_000_000, "uosmo"));

    let admin = chain.init_account(coins(1_000_000_000_000, "uosmo"))?;
    let admin_address = Addr::unchecked(admin.address());

    let cw721_base_contract = Cw721Base::new(chain.clone());
    cw721_base_contract.upload()?;
    cw721_base_contract.call_as(&admin).instantiate(
        &Cw721InstantiateMsg {
            name: String::from("Bad Kids"),
            symbol: String::from("BAD"),
            minter: admin_address.to_string(),
        },
        Some(&admin_address),
        None,
    )?;

    chain.call_as(&admin).commit_any::<MsgCreateDenomResponse>(
        vec![Any {
            type_url: MsgCreateDenom::TYPE_URL.to_string(),
            value: MsgCreateDenom {
                sender: admin_address.to_string(),
                subdenom: SUBDENOM.to_string(),
            }
            .encode_to_vec(),
        }],
        None,
    )?;

    let denom = format!("factory/{}/{}", admin_address.to_string(), SUBDENOM);

    let frac_lockup_contract = FracLockup::new(chain.clone());
    let frac_lockup_code_id = frac_lockup_contract.upload()?.uploaded_code_id()?;

    let frac_lockup_instantiate_msg = FracInstantiateMsg {
        collections: vec![CollectionInput {
            address: cw721_base_contract.addr_str()?,
            tokens: 1000000,
        }],
        denom: denom.clone(),
    };

    let frac_lockup_init_response = chain
        .call_as(&admin)
        .commit_any::<MsgInstantiateContractResponse>(
            vec![Any {
                type_url: MsgInstantiateContract::TYPE_URL.to_string(),
                value: MsgInstantiateContract {
                    sender: admin_address.to_string(),
                    admin: admin_address.to_string(),
                    code_id: frac_lockup_code_id,
                    label: "frac-lockup".to_string(),
                    funds: vec![],
                    msg: cosmwasm_std::to_json_binary(&frac_lockup_instantiate_msg)?.into(),
                }
                .encode_to_vec(),
            }],
            None,
        )?;

    frac_lockup_contract.set_address(&frac_lockup_init_response.instantiated_contract_address()?);

    chain.call_as(&admin).commit_any::<MsgChangeAdminResponse>(
        vec![Any {
            type_url: MsgChangeAdmin::TYPE_URL.to_string(),
            value: MsgChangeAdmin {
                sender: admin_address.to_string(),
                denom: denom.clone(),
                new_admin: frac_lockup_contract.addr_str()?,
            }
            .encode_to_vec(),
        }],
        None,
    )?;

    Ok(TestState {
        admin,
        chain,
        cw721_base: cw721_base_contract,
        frac_lockup: frac_lockup_contract,
        denom,
    })
}

fn mint_cw721(state: TestState, address: String, token_id: &str) {
    let msg = Cw721ExecuteMsg::Mint {
        token_id: token_id.to_string(),
        owner: address,
        token_uri: None,
        extension: Empty {},
    };

    state
        .cw721_base
        .call_as(&state.admin)
        .execute(&msg, None)
        .unwrap();
}

fn send_cw721(state: TestState, sender: Rc<SigningAccount>, recipient: Addr, token_id: &str) {
    let msg = Cw721ExecuteMsg::SendNft {
        contract: recipient.to_string(),
        token_id: token_id.to_string(),
        msg: b"{}".to_vec().into(),
    };

    state
        .cw721_base
        .call_as(&sender)
        .execute(&msg, None)
        .unwrap();
}

#[test]
fn proper_initialization() {
    setup_contracts().unwrap();
}

#[test]
fn try_query_config() {
    let state = setup_contracts().unwrap();
    let config = state
        .frac_lockup
        .query::<ConfigResponse>(&FracQueryMsg::config())
        .unwrap();

    assert_eq!(config.admin, state.admin.address());
    assert_eq!(config.denom, state.denom);
    assert_eq!(config.collections.len(), 1);
    assert_eq!(
        config.collections[0].address,
        state.cw721_base.addr_str().unwrap()
    );
    assert_eq!(config.collections[0].tokens, Uint128::from(1_000_000u128));
}

#[test]
fn try_deposit() {
    let state = setup_contracts().unwrap();

    mint_cw721(state.clone(), state.admin.address(), "1");
    send_cw721(
        state.clone(),
        state.admin.clone(),
        state.frac_lockup.address().unwrap(),
        "1",
    );

    let response = state
        .frac_lockup
        .query::<LockupsReponse>(&FracQueryMsg::LockupsByDepositor {
            depositor: state.admin.address(),
        })
        .unwrap();

    assert_eq!(response.lockups.len(), 1);
    assert_eq!(response.lockups[0].depositor, state.admin.address());
    assert_eq!(
        response.lockups[0].collection_address,
        state.cw721_base.address().unwrap()
    );
    assert_eq!(response.lockups[0].token_id, "1");

    let balance = state
        .chain
        .query_balance(&state.admin.address(), &state.denom)
        .unwrap();
    assert_eq!(balance, Uint128::from(1_000_000u128));
}

#[test]
fn try_withdraw() {
    let state = setup_contracts().unwrap();

    mint_cw721(state.clone(), state.admin.address(), "1");
    send_cw721(
        state.clone(),
        state.admin.clone(),
        state.frac_lockup.address().unwrap(),
        "1",
    );

    let response = state
        .frac_lockup
        .query::<LockupsReponse>(&FracQueryMsg::LockupsByDepositor {
            depositor: state.admin.address(),
        })
        .unwrap();

    assert_eq!(response.lockups.len(), 1);
    assert_eq!(response.lockups[0].depositor, state.admin.address());
    assert_eq!(
        response.lockups[0].collection_address,
        state.cw721_base.address().unwrap()
    );
    assert_eq!(response.lockups[0].token_id, "1");

    let balance = state
        .chain
        .query_balance(&state.admin.address(), &state.denom)
        .unwrap();
    assert_eq!(balance, Uint128::from(1_000_000u128));

    let msg = FracExecuteMsg::Withdraw {
        collection_address: state.cw721_base.addr_str().unwrap(),
        token_id: "1".to_string(),
    };

    state
        .frac_lockup
        .call_as(&state.admin)
        .execute(&msg, Some(&[coin(1_000_000, state.denom.clone())]))
        .unwrap();

    let response = state
        .frac_lockup
        .query::<LockupsReponse>(&FracQueryMsg::LockupsByDepositor {
            depositor: state.admin.address(),
        })
        .unwrap();

    assert_eq!(response.lockups.len(), 0);

    let balance = state
        .chain
        .query_balance(&state.admin.address(), &state.denom)
        .unwrap();
    assert_eq!(balance, Uint128::zero());
}
