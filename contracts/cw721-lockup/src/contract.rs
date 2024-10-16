use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, Addr, Response, StdError, StdResult, SubMsg, Timestamp,
    WasmMsg,
};
use cw2::ContractVersion;
use cw_storage_plus::{IndexedMap, Item, MultiIndex};

use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};
use sylvia::{contract, entry_points};

use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse as Cw721OwnerOfResponse};

use crate::msg::{ConfigResponse, CountResponse, LockupsReponse};
use crate::storage::{Lockup, LockupIndexes};
use crate::{ACTOR_ID, VERSION};

pub struct Cw721LockupContract {
    pub(crate) admin: Item<'static, Addr>,
    pub(crate) lockup_interval: Item<'static, Timestamp>,
    pub(crate) collections: Item<'static, Vec<Addr>>,
    pub(crate) lockup: IndexedMap<'static, &'static str, Lockup, LockupIndexes<'static>>,
}

#[entry_points]
#[contract]
impl Cw721LockupContract {
    pub const fn new() -> Self {
        let indexes = LockupIndexes {
            token: MultiIndex::new(
                |_, d| (d.collection_address.clone(), d.token_id.clone()),
                "lockup",
                "lockup__token",
            ),
            collection: MultiIndex::new(
                |_, d| d.collection_address.clone(),
                "lockup",
                "lockup__collection",
            ),
            owner: MultiIndex::new(|_, d| d.owner.clone(), "lockup", "lockup__owner"),
        };

        Self {
            admin: Item::new("admin"),
            lockup_interval: Item::new("lockup_interval"),
            collections: Item::new("collections"),
            lockup: IndexedMap::new("lockup", indexes),
        }
    }

    #[msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        lockup_interval: Option<Timestamp>,
        collections: Vec<String>,
    ) -> StdResult<Response> {
        let collections: Result<Vec<Addr>, StdError> = collections
            .into_iter()
            .map(|addr| {
                ctx.deps
                    .api
                    .addr_validate(&addr)
                    .map_err(|_| StdError::generic_err("Invalid collection address"))
            })
            .collect();

        match collections {
            Ok(valid_collections) => {
                self.lockup_interval.save(
                    ctx.deps.storage,
                    &lockup_interval
                        .or(Some(Timestamp::from_seconds(0)))
                        .unwrap(),
                )?;
                self.collections
                    .save(ctx.deps.storage, &valid_collections)?;
                self.admin.save(ctx.deps.storage, &ctx.info.sender)?;

                Ok(Response::new())
            }
            Err(err) => {
                return Err(err);
            }
        }
    }

    #[msg(exec)]
    fn update_admin(&self, ctx: ExecCtx, admin: String) -> StdResult<Response> {
        // Admin only
        let old_admin = self.admin.load(ctx.deps.storage).unwrap();
        ensure_eq!(
            old_admin,
            ctx.info.sender,
            StdError::generic_err("Unauthorized")
        );

        // Update the admin
        let admin = ctx.deps.api.addr_validate(&admin)?;
        self.admin.save(ctx.deps.storage, &admin).unwrap();

        Ok(Response::new()
            .add_attribute("method", "update_admin")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("old_admin", old_admin.to_string())
            .add_attribute("new_admin", admin.to_string()))
    }

    #[msg(exec)]
    fn update_config(
        &self,
        ctx: ExecCtx,
        lockup_interval: Option<Timestamp>,
        collections: Vec<String>,
    ) -> StdResult<Response> {
        // Admin only
        let old_admin = self.admin.load(ctx.deps.storage).unwrap();
        ensure_eq!(
            old_admin,
            ctx.info.sender,
            StdError::generic_err("Unauthorized")
        );

        // Verify collections addresses
        let collections: Vec<Addr> = collections
            .into_iter()
            .map(|addr| ctx.deps.api.addr_validate(&addr).unwrap())
            .collect();

        // Save the new config
        self.lockup_interval.save(
            ctx.deps.storage,
            &lockup_interval
                .or(Some(Timestamp::from_seconds(0)))
                .unwrap(),
        )?;
        self.collections.save(ctx.deps.storage, &collections)?;

        Ok(Response::new()
            .add_attribute("method", "update_config")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute(
                "lockup_interval",
                lockup_interval
                    .or(Some(Timestamp::from_seconds(0)))
                    .unwrap()
                    .to_string(),
            )
            .add_attribute(
                "collections",
                collections
                    .iter()
                    .map(|addr| addr.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            ))
    }

    #[msg(exec)]
    fn receive_nft(&self, ctx: ExecCtx, sender: String, token_id: String) -> StdResult<Response> {
        let collection_address = ctx.info.sender;

        // Verify that the collection is supported
        let collections = self.collections.load(ctx.deps.storage)?;
        ensure!(
            collections.contains(&collection_address),
            StdError::generic_err("Collection is not supported")
        );

        // Query the owner of the NFT
        let cw721_owner_response: Cw721OwnerOfResponse = ctx
            .deps
            .querier
            .query_wasm_smart(
                collection_address.clone(),
                &Cw721QueryMsg::OwnerOf {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )
            .map_err(|error| error)?;

        // Verify that the contract is the owner of the NFT
        ensure_eq!(
            cw721_owner_response.owner,
            ctx.env.contract.address.to_string(),
            StdError::generic_err("Token was not transferred to contract")
        );

        // Save a new lockup entry
        let owner = ctx.deps.api.addr_validate(&sender)?;
        let lockup_interval = self.lockup_interval.load(ctx.deps.storage)?;
        let locked_until = ctx.env.block.time.plus_seconds(lockup_interval.seconds());

        let lockup = Lockup::new(
            owner.clone(),
            collection_address.clone(),
            token_id.clone(),
            ctx.env.block.time,
            locked_until.clone(),
        );

        self.lockup
            .save(ctx.deps.storage, "lockup__depositor", &lockup)?;

        Ok(Response::new()
            .add_attribute("method", "deposit")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("collection_address", collection_address.to_string())
            .add_attribute("token_id", token_id)
            .add_attribute("owner", owner.to_string())
            .add_attribute("locked_until", locked_until.to_string()))
    }

    #[msg(exec)]
    fn withdraw(
        &self,
        ctx: ExecCtx,
        collection_address: String,
        token_id: String,
    ) -> StdResult<Response> {
        let sender = ctx.info.sender;
        let collection_address = ctx.deps.api.addr_validate(&collection_address)?;

        // Retrieve the lockup entry for the NFT
        // @josefleventon: Usage of UniqueIndex here is hindered by an error from cw-storage-plus requiring prefix() to receive only an Addr struct. Contact CosmWasm team to inform them of this.
        let lockup_key = (collection_address.clone(), token_id.clone());
        let lockup_data = self
            .lockup
            .idx
            .token
            .prefix(lockup_key.clone())
            .range(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        let lockup = match lockup_data.first() {
            Some(lockup) => lockup,
            None => {
                return Err(StdError::generic_err("Lockup entry not found"));
            }
        };

        // Verify that the sender is the owner of the NFT
        ensure_eq!(
            lockup.1.owner,
            sender,
            StdError::generic_err("Sender is not the owner of the NFT")
        );
        // Verify that the lockup period has passed
        ensure!(
            ctx.env.block.time > lockup.1.locked_until,
            StdError::generic_err("Lockup period has not passed")
        );

        // Delete the lockup entry
        self.lockup.remove(ctx.deps.storage, &lockup.0)?;

        // Send the NFT back to the owner
        let msg = Cw721ExecuteMsg::TransferNft {
            recipient: lockup.1.owner.to_string(),
            token_id: lockup.1.token_id.to_string(),
        };
        let cw721_msg = WasmMsg::Execute {
            contract_addr: lockup.1.collection_address.to_string(),
            msg: to_json_binary(&msg)?,
            funds: vec![],
        };

        Ok(Response::new()
            .add_submessage(SubMsg::new(cw721_msg))
            .add_attribute("method", "withdraw")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("collection_address", collection_address)
            .add_attribute("token_id", token_id)
            .add_attribute("owner", lockup.1.owner.to_string()))
    }

    #[msg(query)]
    fn count(&self, ctx: QueryCtx) -> StdResult<CountResponse> {
        let count = self
            .lockup
            .keys(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .count() as u128;
        Ok(CountResponse { count })
    }

    #[msg(query)]
    fn contract_type(&self, _ctx: QueryCtx) -> StdResult<ContractVersion> {
        Ok(ContractVersion {
            contract: ACTOR_ID.to_string(),
            version: VERSION.to_string(),
        })
    }

    #[msg(query)]
    fn lockup_by_token(
        &self,
        ctx: QueryCtx,
        collection_address: String,
        token_id: String,
    ) -> StdResult<Lockup> {
        let collection_address = ctx.deps.api.addr_validate(&collection_address)?;
        let lockup_key = (collection_address.clone(), token_id.clone());
        let lockup_data = self
            .lockup
            .idx
            .token
            .prefix(lockup_key.clone())
            .range(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        let lockup = match lockup_data.first() {
            Some(lockup) => lockup,
            None => {
                return Err(StdError::generic_err("Lockup entry not found"));
            }
        };
        Ok(lockup.1.clone())
    }

    #[msg(query)]
    fn lockups_by_owner(&self, ctx: QueryCtx, owner: String) -> StdResult<LockupsReponse> {
        let owner = ctx.deps.api.addr_validate(&owner)?;
        let lockups = self
            .lockup
            .idx
            .owner
            .prefix(owner.clone())
            .range(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|res| res.map(|item| item.1))
            .collect::<StdResult<Vec<_>>>()?;
        Ok(LockupsReponse { lockups })
    }

    #[msg(query)]
    fn lockups_by_collection(
        &self,
        ctx: QueryCtx,
        collection_address: String,
    ) -> StdResult<LockupsReponse> {
        let collection_address = ctx.deps.api.addr_validate(&collection_address)?;
        let lockups = self
            .lockup
            .idx
            .collection
            .prefix(collection_address.clone())
            .range(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|res| res.map(|item| item.1))
            .collect::<StdResult<Vec<_>>>()?;
        Ok(LockupsReponse { lockups })
    }

    #[msg(query)]
    fn config(&self, ctx: QueryCtx) -> StdResult<ConfigResponse> {
        let admin = self.admin.load(ctx.deps.storage)?;
        let lockup_interval = self.lockup_interval.load(ctx.deps.storage)?;
        let collections = self.collections.load(ctx.deps.storage)?;
        Ok(ConfigResponse {
            admin,
            lockup_interval,
            collections,
        })
    }
}
