use cw_orch::{interface, prelude::*};

use fixed_stake::contract::entry_points::{execute, instantiate, query};
pub use fixed_stake::contract::{ExecMsg as ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, cosmwasm_std::Empty,  id = fixed_stake::CONTRACT)]
pub struct FixedStake;

impl<Chain> Uploadable for FixedStake<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path(fixed_stake::CONTRACT)
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
