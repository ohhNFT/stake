use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, Addr, BankMsg, Coin, Response, StdError, StdResult, SubMsg,
    Timestamp, Uint128, WasmMsg,
};
use cw2::ContractVersion;
use cw_storage_plus::{IndexedMap, Item, MultiIndex};

use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};
use sylvia::{contract, entry_points};

use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse as Cw721OwnerOfResponse};

use crate::helpers::{burn, mint_to};
use crate::msg::{ConfigResponse, CountResponse, LockupsReponse};
use crate::storage::{Collection, CollectionInput, Lockup, LockupIndexes};
use crate::{ACTOR_ID, VERSION};

pub struct FracLockupContract {
    pub(crate) admin: Item<'static, Addr>,
    pub(crate) denom: Item<'static, String>,
    pub(crate) collections: Item<'static, Vec<Collection>>,
    pub(crate) lockup: IndexedMap<'static, &'static str, Lockup, LockupIndexes<'static>>,
}

#[entry_points]
#[contract]
impl FracLockupContract {
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
            depositor: MultiIndex::new(|_, d| d.depositor.clone(), "lockup", "lockup__depositor"),
        };

        Self {
            admin: Item::new("admin"),
            denom: Item::new("denom"),
            collections: Item::new("collections"),
            lockup: IndexedMap::new("lockup", indexes),
        }
    }

    #[msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        denom: String,
        collections: Vec<CollectionInput>,
    ) -> StdResult<Response> {
        let collections: Vec<Collection> = collections
            .into_iter()
            .map(|collection| Collection {
                address: ctx.deps.api.addr_validate(&collection.address).unwrap(),
                tokens: Uint128::from(collection.tokens),
            })
            .collect();

        // Verify that the denom begins with `factory/`
        ensure!(
            denom.starts_with("factory/"),
            StdError::generic_err("Denom must be a token factory token")
        );

        self.denom.save(ctx.deps.storage, &denom)?;
        self.collections.save(ctx.deps.storage, &collections)?;
        self.admin.save(ctx.deps.storage, &ctx.info.sender)?;

        Ok(Response::new())
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
    fn append_collection(&self, ctx: ExecCtx, collection: CollectionInput) -> StdResult<Response> {
        // Admin only
        let old_admin = self.admin.load(ctx.deps.storage).unwrap();
        ensure_eq!(
            old_admin,
            ctx.info.sender,
            StdError::generic_err("Unauthorized")
        );

        // Verify collections addresses
        let address = ctx.deps.api.addr_validate(&collection.address)?;
        let tokens = Uint128::from(collection.tokens);

        // Verify that there is not already a collection with this address
        let collections = self.collections.load(ctx.deps.storage)?;
        ensure!(
            !collections
                .iter()
                .any(|collection| collection.address == address),
            StdError::generic_err("Collection already exists")
        );

        // Save the new collection
        let new_collections: Vec<Collection> = collections
            .iter()
            .chain(vec![Collection { address, tokens }].iter())
            .cloned()
            .collect();

        // Save the new collections
        self.collections.save(ctx.deps.storage, &new_collections)?;

        Ok(Response::new()
            .add_attribute("method", "append_collection")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute(
                "collections",
                collections
                    .iter()
                    .map(|collection| collection.address.to_string())
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
            collections
                .iter()
                .any(|collection| collection.address == collection_address),
            StdError::generic_err("Collection is not supported")
        );

        let collection = collections
            .iter()
            .find(|collection| collection.address == collection_address)
            .unwrap();

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
        let depositor = ctx.deps.api.addr_validate(&sender)?;

        let lockup = Lockup::new(
            depositor.clone(),
            collection_address.clone(),
            token_id.clone(),
            ctx.env.block.time,
        );

        self.lockup
            .save(ctx.deps.storage, "lockup__depositor", &lockup)?;

        // Mint tokens to depositor
        let mint_msg = mint_to(
            ctx.env.clone(),
            depositor.to_string(),
            Coin {
                denom: self.denom.load(ctx.deps.storage)?,
                amount: collection.tokens.clone(),
            },
        );

        Ok(Response::new()
            .add_submessage(SubMsg::new(mint_msg))
            .add_attribute("method", "deposit")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("collection_address", collection_address.to_string())
            .add_attribute("token_id", token_id)
            .add_attribute("depositor", depositor.to_string()))
    }

    #[msg(exec)]
    fn withdraw(
        &self,
        ctx: ExecCtx,
        collection_address: String,
        token_id: String,
    ) -> StdResult<Response> {
        // Verify that the amount of funds sent is over 0
        ensure!(
            !ctx.info.funds.is_empty(),
            StdError::generic_err("No funds sent")
        );
        ensure!(
            ctx.info.funds[0].amount > Uint128::zero(),
            StdError::generic_err("Funds sent must be greater than 0")
        );

        // Verify that only one token type was sent
        ensure_eq!(
            ctx.info.funds.len(),
            1,
            StdError::generic_err("Only one token type can be sent")
        );

        // Verify that the funds sent are in the correct token
        let denom = self.denom.load(ctx.deps.storage)?;
        ensure_eq!(
            ctx.info.funds[0].denom.as_str(),
            denom.as_str(),
            StdError::generic_err("Unsupported token sent")
        );

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

        // Verify that the appropriate amount of funds was sent
        let collections = self.collections.load(ctx.deps.storage)?;
        let collection = collections
            .iter()
            .find(|collection| collection.address == collection_address)
            .unwrap();
        let amount = collection.tokens.clone();
        ensure_eq!(
            ctx.info.funds[0].amount,
            amount,
            StdError::generic_err("Incorrect amount of funds sent")
        );

        // Delete the lockup entry
        self.lockup.remove(ctx.deps.storage, &lockup.0)?;

        // Burn the tokens
        let burn_msg = burn(ctx.env.clone(), ctx.info.funds[0].clone());

        // Send the NFT to the message sender
        let msg = Cw721ExecuteMsg::TransferNft {
            recipient: sender.to_string(),
            token_id: lockup.1.token_id.to_string(),
        };
        let cw721_msg = WasmMsg::Execute {
            contract_addr: lockup.1.collection_address.to_string(),
            msg: to_json_binary(&msg)?,
            funds: vec![],
        };

        Ok(Response::new()
            .add_submessage(SubMsg::new(cw721_msg))
            .add_submessage(SubMsg::new(burn_msg))
            .add_attribute("method", "withdraw")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("collection_address", collection_address)
            .add_attribute("token_id", token_id)
            .add_attribute("sent_to", sender.to_string()))
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
    fn lockups_by_depositor(&self, ctx: QueryCtx, depositor: String) -> StdResult<LockupsReponse> {
        let depositor = ctx.deps.api.addr_validate(&depositor)?;
        let lockups = self
            .lockup
            .idx
            .depositor
            .prefix(depositor.clone())
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
        let denom = self.denom.load(ctx.deps.storage)?;
        let collections = self.collections.load(ctx.deps.storage)?;
        Ok(ConfigResponse {
            admin,
            denom,
            collections,
        })
    }
}
