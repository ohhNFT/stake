pub const CONTRACT: &str = "cw721_lockup";
pub const ACTOR_ID: &str = "cw721";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod contract;
pub mod msg;
pub mod storage;
