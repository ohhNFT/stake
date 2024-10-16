pub const CONTRACT: &str = "crates.io:fixed-stake";
pub const ACTOR_ID: &str = "fixed";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod contract;
pub mod msg;
pub mod storage;

#[cfg(test)]
pub mod multitest;
