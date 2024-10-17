use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, MultiIndex};

#[cw_serde]
pub struct CollectionInput {
    pub address: String,
    pub tokens: u128,
}

#[cw_serde]
pub struct Collection {
    pub address: Addr,
    pub tokens: Uint128,
}

#[cw_serde]
pub struct Lockup {
    pub depositor: Addr,
    pub collection_address: Addr,
    pub token_id: String,
    pub locked_since: Timestamp,
}

impl Lockup {
    pub fn new(
        depositor: Addr,
        collection_address: Addr,
        token_id: String,
        locked_since: Timestamp,
    ) -> Self {
        Self {
            depositor,
            collection_address,
            token_id,
            locked_since,
        }
    }
}

type Token = (Addr, String);

pub struct LockupIndexes<'a> {
    pub token: MultiIndex<'a, Token, Lockup, String>,
    pub collection: MultiIndex<'a, Addr, Lockup, String>,
    pub depositor: MultiIndex<'a, Addr, Lockup, String>,
}

impl<'a> IndexList<Lockup> for LockupIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Lockup>> + '_> {
        let v: Vec<&dyn Index<Lockup>> = vec![&self.token, &self.depositor];
        Box::new(v.into_iter())
    }
}
