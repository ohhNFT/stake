use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Index, IndexList, MultiIndex};

#[cw_serde]
pub struct Lockup {
    pub owner: Addr,
    pub collection_address: Addr,
    pub token_id: String,
    pub locked_since: Timestamp,
    pub locked_until: Timestamp,
}

impl Lockup {
    pub fn new(
        owner: Addr,
        collection_address: Addr,
        token_id: String,
        locked_since: Timestamp,
        locked_until: Timestamp,
    ) -> Self {
        Self {
            owner,
            collection_address,
            token_id,
            locked_since,
            locked_until,
        }
    }
}

type Token = (Addr, String);

pub struct LockupIndexes<'a> {
    pub token: MultiIndex<'a, Token, Lockup, String>,
    pub collection: MultiIndex<'a, Addr, Lockup, String>,
    pub owner: MultiIndex<'a, Addr, Lockup, String>,
}

impl<'a> IndexList<Lockup> for LockupIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Lockup>> + '_> {
        let v: Vec<&dyn Index<Lockup>> = vec![&self.token, &self.owner];
        Box::new(v.into_iter())
    }
}
