use cw_orch::{interface, prelude::*};

use cw721_lockup::contract::entry_points::{execute, instantiate, query};

pub use cw721::Cw721QueryMsg as QueryMsg;
pub use cw721_base::{ExecuteMsg, InstantiateMsg};

pub type Cw721ExecuteMsg = ExecuteMsg<cosmwasm_std::Empty, cosmwasm_std::Empty>;

#[interface(
    InstantiateMsg,
    Cw721ExecuteMsg,
    QueryMsg,
    cosmwasm_std::Empty,
    id = "cw721_base"
)]
pub struct Cw721Base;

impl<Chain> Uploadable for Cw721Base<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw721_base")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
