use cosmwasm_std::{
    coin, ensure, ensure_eq, Addr, BankMsg, Response, StdError, StdResult, SubMsg, Timestamp,
    Uint128,
};
use cw_storage_plus::{Item, Map};

use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};
use sylvia::{contract, entry_points};

use crate::msg::{ConfigResponse, ContractTypeResponse, CountResponse};
use crate::storage::Lockup;

pub struct NativeLockupContract {
    pub(crate) admin: Item<'static, Addr>,
    pub(crate) token: Item<'static, String>,
    pub(crate) lockup_interval: Item<'static, Timestamp>,
    pub(crate) lockup: Map<'static, Addr, Lockup>,
}

#[entry_points]
#[contract]
impl NativeLockupContract {
    pub const fn new() -> Self {
        Self {
            admin: Item::new("admin"),
            token: Item::new("token"),
            lockup_interval: Item::new("lockup_interval"),
            lockup: Map::new("lockup"),
        }
    }

    #[msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        token: String,
        lockup_interval: Timestamp,
    ) -> StdResult<Response> {
        self.token.save(ctx.deps.storage, &token)?;
        self.lockup_interval
            .save(ctx.deps.storage, &lockup_interval)?;
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
            .add_attribute("old_admin", old_admin.to_string())
            .add_attribute("new_admin", admin.to_string()))
    }

    #[msg(exec)]
    fn update_config(
        &self,
        ctx: ExecCtx,
        token: String,
        lockup_interval: Timestamp,
    ) -> StdResult<Response> {
        // Admin only
        let old_admin = self.admin.load(ctx.deps.storage).unwrap();
        ensure_eq!(
            old_admin,
            ctx.info.sender,
            StdError::generic_err("Unauthorized")
        );
        // Save the new config
        self.token.save(ctx.deps.storage, &token)?;
        self.lockup_interval
            .save(ctx.deps.storage, &lockup_interval)?;

        Ok(Response::new()
            .add_attribute("method", "update_config")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("token", token)
            .add_attribute("lockup_interval", lockup_interval.to_string()))
    }

    #[msg(exec)]
    fn deposit(&self, ctx: ExecCtx) -> StdResult<Response> {
        // Verify that the amount of funds sent is over 0
        ensure!(
            !ctx.info.funds.is_empty(),
            StdError::generic_err("No funds sent")
        );
        ensure!(
            ctx.info.funds[0].amount > Uint128::zero(),
            StdError::generic_err("Funds sent must be greater than 0")
        );

        // Verify that the funds sent are in the correct token
        let token = self.token.load(ctx.deps.storage)?;
        ensure_eq!(
            ctx.info.funds[0].denom.as_str(),
            token.as_str(),
            StdError::generic_err("Unsupported token sent")
        );

        // Check if there is already a lockup for this user
        let existing_lockup = self
            .lockup
            .may_load(ctx.deps.storage, ctx.info.sender.clone())?;
        let lockup_interval = self.lockup_interval.load(ctx.deps.storage)?;

        // If there is no lockup, create a new one
        // If there is one, append the funds to the existing lockup
        match existing_lockup {
            Some(lockup) => {
                let mut new_lockup = lockup;
                new_lockup.amount += ctx.info.funds[0].amount;
                new_lockup.locked_until =
                    ctx.env.block.time.plus_seconds(lockup_interval.seconds());
                self.lockup
                    .save(ctx.deps.storage, ctx.info.sender.clone(), &new_lockup)?;
            }
            None => {
                let lockup = Lockup::new(
                    ctx.info.funds[0].amount,
                    ctx.env.block.time,
                    ctx.env.block.time.plus_seconds(lockup_interval.seconds()),
                );

                self.lockup
                    .save(ctx.deps.storage, ctx.info.sender.clone(), &lockup)?;
            }
        }

        let new_lockup = self
            .lockup
            .load(ctx.deps.storage, ctx.info.sender.clone())?;

        Ok(Response::new()
            .add_attribute("method", "deposit")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("sender", ctx.info.sender.to_string())
            .add_attribute("amount", new_lockup.amount.to_string())
            .add_attribute("locked_until", new_lockup.locked_until.to_string()))
    }

    #[msg(exec)]
    fn withdraw(&self, ctx: ExecCtx, amount: Option<Uint128>) -> StdResult<Response> {
        let lockup = self
            .lockup
            .load(ctx.deps.storage, ctx.info.sender.clone())?;

        // If the lockup has not expired, return an error
        ensure!(
            ctx.env.block.time > lockup.locked_until,
            StdError::generic_err("Lockup period has not passed")
        );

        let token = self.token.load(ctx.deps.storage)?;

        // If there is no amount specified, send the full amount
        let amount = amount.unwrap_or(lockup.amount);
        let msg = BankMsg::Send {
            to_address: ctx.info.sender.to_string(),
            amount: vec![coin(amount.u128(), token.clone())],
        };
        let send_msg = SubMsg::new(msg);

        // Subtract the amount from the lockup
        // or remove the lockup if the entire balance was withdrawn
        if amount == lockup.amount {
            self.lockup
                .remove(ctx.deps.storage, ctx.info.sender.clone());
        } else {
            self.lockup.update(
                ctx.deps.storage,
                ctx.info.sender.clone(),
                |lockup| match lockup {
                    Some(lockup) => Ok(Lockup::new(
                        lockup.amount - amount,
                        lockup.locked_since,
                        lockup.locked_until,
                    )),
                    None => Err(StdError::generic_err("Lockup not found")),
                },
            )?;
        }

        // Send the funds to the user
        let res = Response::new()
            .add_submessage(send_msg)
            .add_attribute("method", "withdraw")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("sender", ctx.info.sender.to_string())
            .add_attribute("denom", token)
            .add_attribute("amount", lockup.amount.to_string());

        Ok(res)
    }

    #[msg(query)]
    fn count(&self, ctx: QueryCtx) -> StdResult<CountResponse> {
        // Get all amounts from `count` Map
        let lockups = self
            .lockup
            .range(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|res| res.map(|item| item.1.amount.u128()))
            .collect::<StdResult<Vec<_>>>()?;

        // Get sum of items in lockups
        let count = lockups.iter().sum::<u128>();

        Ok(CountResponse { count })
    }

    #[msg(query)]
    fn contract_type(&self, _ctx: QueryCtx) -> StdResult<ContractTypeResponse> {
        Ok(ContractTypeResponse {
            contract_type: "native".to_string(),
        })
    }

    #[msg(query)]
    fn lockup(&self, ctx: QueryCtx, address: String) -> StdResult<Lockup> {
        let address = ctx.deps.api.addr_validate(&address)?;
        let lockup = self.lockup.load(ctx.deps.storage, address)?;
        Ok(lockup)
    }

    #[msg(query)]
    fn config(&self, ctx: QueryCtx) -> StdResult<ConfigResponse> {
        let admin = self.admin.load(ctx.deps.storage)?;
        let lockup_interval = self.lockup_interval.load(ctx.deps.storage)?;
        let token = self.token.load(ctx.deps.storage)?;
        Ok(ConfigResponse {
            admin,
            lockup_interval,
            token,
        })
    }
}
