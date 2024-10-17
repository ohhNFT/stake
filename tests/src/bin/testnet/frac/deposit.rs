use cw_orch::prelude::*;

use tests::chains::ELGAFAR_1;
use tests::interface::cw721_base::{Cw721Base, Cw721ExecuteMsg};

pub fn main() {
    dotenv::dotenv().unwrap();
    env_logger::init();

    let daemon_res = Daemon::builder(ELGAFAR_1).build();
    assert!(daemon_res.is_ok());
    let daemon = daemon_res.unwrap();

    let frac_lockup_address_res = daemon.state().get_address("frac_lockup");
    assert!(frac_lockup_address_res.is_ok());
    let frac_lockup_address = frac_lockup_address_res.unwrap();

    let cw721_base_address_res = daemon.state().get_address("cw721_base");
    assert!(cw721_base_address_res.is_ok());
    let cw721_base_address = cw721_base_address_res.unwrap();

    let cw721_base = Cw721Base::new(daemon.clone());
    cw721_base.set_address(&cw721_base_address);

    let msg = Cw721ExecuteMsg::SendNft {
        contract: frac_lockup_address.to_string(),
        token_id: "1".to_string(),
        msg: b"{}".to_vec().into(),
    };

    let exec_res = cw721_base.execute(&msg, None);
    assert!(exec_res.is_ok());
}
