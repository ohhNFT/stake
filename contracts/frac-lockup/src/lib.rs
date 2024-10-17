pub const CONTRACT: &str = "frac_lockup";
pub const ACTOR_ID: &str = "frac";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod contract;
pub mod helpers;
pub mod msg;
pub mod storage;
