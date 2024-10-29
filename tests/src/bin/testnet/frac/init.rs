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
use rand::Rng;
use std::iter;

use tests::interface::cw721_base::{
    Cw721Base, Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg,
};
use tests::interface::frac_lockup::{FracLockup, InstantiateMsg as FracInstantiateMsg};

use tests::chains::ELGAFAR_1;

fn generate_str(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    let one_char = || CHARSET[rng.gen_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(len).collect()
}

pub fn main() {
    dotenv::dotenv().unwrap();
    env_logger::init();

    let daemon_res = Daemon::builder(ELGAFAR_1).build();
    assert!(daemon_res.is_ok());
    let daemon = daemon_res.unwrap();

    let subdenom = generate_str(5);
    let set_res = daemon.state().set("custom", "subdenom", subdenom.clone());
    assert!(set_res.is_ok());

    let cw721_base = Cw721Base::new(daemon.clone());

    let cw721_upload_res = cw721_base.upload();
    assert!(cw721_upload_res.is_ok());

    let cw721_init_res = cw721_base.instantiate(
        &Cw721InstantiateMsg {
            name: String::from("WAU Test Collection"),
            symbol: subdenom.clone(),
            minter: daemon.sender_addr().to_string(),
        },
        Some(&daemon.sender_addr()),
        None,
    );
    assert!(cw721_init_res.is_ok());

    let create_denom_res = daemon.commit_any::<MsgCreateDenomResponse>(
        vec![Any {
            type_url: MsgCreateDenom::TYPE_URL.to_string(),
            value: MsgCreateDenom {
                sender: daemon.sender_addr().to_string(),
                subdenom: subdenom.clone(),
            }
            .encode_to_vec(),
        }],
        None,
    );
    assert!(create_denom_res.is_ok());

    let denom = format!(
        "factory/{}/{}",
        daemon.sender_addr().to_string(),
        subdenom.clone()
    );

    let set_res = daemon.state().set("custom", "denom", denom.clone());
    assert!(set_res.is_ok());

    let frac_lockup = FracLockup::new(daemon.clone());

    let frac_lockup_init_res = frac_lockup.upload();
    assert!(frac_lockup_init_res.is_ok());

    let frac_lockup_code_id_res = frac_lockup_init_res.unwrap().uploaded_code_id();
    assert!(frac_lockup_code_id_res.is_ok());
    let frac_lockup_code_id = frac_lockup_code_id_res.unwrap();

    let frac_lockup_instantiate_msg = FracInstantiateMsg {
        collections: vec![CollectionInput {
            address: cw721_base.addr_str().unwrap(),
            tokens: 1000000,
        }],
        denom: denom.clone(),
    };

    let frac_lockup_init_res = daemon.commit_any::<MsgInstantiateContractResponse>(
        vec![Any {
            type_url: MsgInstantiateContract::TYPE_URL.to_string(),
            value: MsgInstantiateContract {
                sender: daemon.sender().address().to_string(),
                admin: daemon.sender().address().to_string(),
                code_id: frac_lockup_code_id,
                label: "frac-lockup".to_string(),
                funds: vec![],
                msg: cosmwasm_std::to_json_binary(&frac_lockup_instantiate_msg)
                    .unwrap()
                    .into(),
            }
            .encode_to_vec(),
        }],
        None,
    );
    assert!(frac_lockup_init_res.is_ok());

    let frac_lockup_address_res = frac_lockup_init_res
        .unwrap()
        .instantiated_contract_address();
    assert!(frac_lockup_address_res.is_ok());
    let frac_lockup_address = frac_lockup_address_res.unwrap();

    frac_lockup.set_address(&frac_lockup_address);

    let change_admin_res = daemon.commit_any::<MsgChangeAdminResponse>(
        vec![Any {
            type_url: MsgChangeAdmin::TYPE_URL.to_string(),
            value: MsgChangeAdmin {
                sender: daemon.sender().address().to_string(),
                denom: denom.clone(),
                new_admin: frac_lockup_address.to_string(),
            }
            .encode_to_vec(),
        }],
        None,
    );
    assert!(change_admin_res.is_ok());

    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "1".to_string(),
        owner: daemon.sender().address().to_string(),
        token_uri: None,
        extension: Empty {},
    };

    let mint_res = cw721_base.execute(&mint_msg, None);
    assert!(mint_res.is_ok());
}
