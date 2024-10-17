use cw_orch::{interface, prelude::*};

use frac_lockup::contract::entry_points::{execute, instantiate, query};
pub use frac_lockup::contract::{ExecMsg as ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, cosmwasm_std::Empty,  id = frac_lockup::CONTRACT)]
pub struct FracLockup;

impl<Chain> Uploadable for FracLockup<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path(frac_lockup::CONTRACT)
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
