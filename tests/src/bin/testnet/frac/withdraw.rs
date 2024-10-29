use cw_orch::{daemon::TxSender, prelude::*};
use frac_lockup::storage::CollectionInput;
use osmosis_test_tube::osmosis_std::types::{
    cosmwasm::wasm::v1::{MsgInstantiateContract, MsgInstantiateContractResponse},
    osmosis::tokenfactory::v1beta1::{
        MsgChangeAdmin, MsgChangeAdminResponse, MsgCreateDenom, MsgCreateDenomResponse,
    },
};
use prost::Message;
use prost_types::Any;

use tests::interface::cw721_base::{
    Cw721Base, Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg,
};
use tests::interface::frac_lockup::{
    ExecuteMsg as FracExecuteMsg, FracLockup, InstantiateMsg as FracInstantiateMsg,
};

use tests::chains::ELGAFAR_1;

pub fn main() {
    dotenv::dotenv().unwrap();
    env_logger::init();

    let daemon_res = Daemon::builder(ELGAFAR_1).build();
    assert!(daemon_res.is_ok());
    let daemon = daemon_res.unwrap();

    let custom_state = daemon.state().get("custom");
    assert!(custom_state.is_ok());
    let state = custom_state.unwrap();
    let denom = state["denom"].as_str().unwrap();

    let frac_lockup_address_res = daemon.state().get_address("frac_lockup");
    assert!(frac_lockup_address_res.is_ok());
    let frac_lockup_address = frac_lockup_address_res.unwrap();

    let cw721_base_address_res = daemon.state().get_address("cw721_base");
    assert!(cw721_base_address_res.is_ok());
    let cw721_base_address = cw721_base_address_res.unwrap();

    let frac_lockup = FracLockup::new(daemon.clone());
    frac_lockup.set_address(&frac_lockup_address);

    let msg = FracExecuteMsg::Withdraw {
        collection_address: cw721_base_address.to_string(),
        token_id: "1".to_string(),
    };

    let exec_res = frac_lockup.execute(
        &msg,
        Some(&[Coin {
            denom: denom.to_string(),
            amount: 1000000u128.into(),
        }]),
    );
    assert!(exec_res.is_ok());
}
