use std::str::FromStr;

use cosmwasm_std::{
    coin, ensure, ensure_eq, Addr, BankMsg, Decimal, Response, StdError, StdResult, SubMsg,
    Timestamp, Uint128,
};
use cw_storage_plus::{Item, Map};

use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};
use sylvia::{contract, entry_points};

use crate::msg::ConfigResponse;
use crate::storage::{LockupInfo, Stake};

pub struct FixedStakeContract {
    pub admin: Item<'static, Addr>,
    pub lockup_contract: Item<'static, Addr>,
    /// Time interval between reward distributions
    pub distribution_interval: Item<'static, Timestamp>,
    /// Reward denomination (e.g. `ustars`, must be a factory token for `inflation-stake`)
    pub reward_denom: Item<'static, String>,
    /// Total rewards to distribute
    pub total_rewards: Item<'static, Uint128>,
    /// Reward distribution start time
    pub start_time: Item<'static, Timestamp>,
    /// Staking claim information (key type depends on lockup contract type)
    pub staking: Map<'static, (Addr, String), Stake>,
    /// Reward distribution end time (`fixed-stake` only)
    pub end_time: Item<'static, Timestamp>,
}

#[entry_points]
#[contract]
impl FixedStakeContract {
    pub const fn new() -> Self {
        Self {
            admin: Item::new("admin"),
            lockup_contract: Item::new("lockup_contract"),
            distribution_interval: Item::new("distribution_interval"),
            reward_denom: Item::new("reward_denom"),
            total_rewards: Item::new("total_rewards"),
            start_time: Item::new("start_time"),
            staking: Map::new("staking"),
            end_time: Item::new("end_time"),
        }
    }

    #[msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        lockup_contract: String,
        distribution_interval: Timestamp,
        reward_denom: String,
        total_rewards: Uint128,
        start_time: Timestamp,
        end_time: Timestamp,
    ) -> StdResult<Response> {
        ensure!(
            end_time > start_time,
            StdError::generic_err("End time must be after start time")
        );

        let lockup_contract = ctx.deps.api.addr_validate(&lockup_contract)?;

        // Query `contract_type` from lockup_contract to verify validity
        let query_msg = native_lockup::contract::QueryMsg::ContractType {};
        let contract_type_response: native_lockup::msg::ContractTypeResponse = ctx
            .deps
            .querier
            .query_wasm_smart(lockup_contract.clone(), &query_msg)
            .map_err(|error| error)?;

        if contract_type_response.contract_type != "native"
            && contract_type_response.contract_type != "cw721"
        {
            return Err(StdError::generic_err("Invalid lockup contract type"));
        }

        self.admin.save(ctx.deps.storage, &ctx.info.sender)?;
        self.lockup_contract
            .save(ctx.deps.storage, &lockup_contract)?;
        self.distribution_interval
            .save(ctx.deps.storage, &distribution_interval)?;
        self.reward_denom.save(ctx.deps.storage, &reward_denom)?;
        self.total_rewards.save(ctx.deps.storage, &total_rewards)?;
        self.start_time.save(ctx.deps.storage, &start_time)?;
        self.end_time.save(ctx.deps.storage, &end_time)?;

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
    fn claim_rewards(&self, ctx: ExecCtx, of: (String, String)) -> StdResult<Response> {
        let of_address = ctx.deps.api.addr_validate(&of.0)?;
        let claimer = (of_address, of.1);

        let staking = self.staking.may_load(ctx.deps.storage, claimer.clone())?;

        let reward_denom = self.reward_denom.load(ctx.deps.storage)?;
        let total_rewards = self.total_rewards.load(ctx.deps.storage)?;
        let start_time = self.start_time.load(ctx.deps.storage)?;
        let end_time = self.end_time.load(ctx.deps.storage)?;
        let distribution_interval = self.distribution_interval.load(ctx.deps.storage)?;

        ensure!(
            ctx.env.block.time > start_time,
            StdError::generic_err("Reward distribution period has not started")
        );
        ensure!(
            ctx.env.block.time < end_time,
            StdError::generic_err("Reward distribution period has ended")
        );

        // Retrieve lockup contract type
        let lockup_contract = self.lockup_contract.load(ctx.deps.storage)?;

        // Make `contract_type` query to lockup_contract
        let query_msg = cw721_lockup::contract::QueryMsg::ContractType {};
        let contract_type_response: cw721_lockup::msg::ContractTypeResponse = ctx
            .deps
            .querier
            .query_wasm_smart(lockup_contract.clone(), &query_msg)
            .map_err(|error| error)?;

        let (lockup, count, claimer_token_id): (LockupInfo, u128, String) =
            match contract_type_response.contract_type.as_str() {
                "native" => {
                    // Verify that claimer.0 is the sender
                    ensure_eq!(
                        claimer.0,
                        ctx.info.sender,
                        StdError::generic_err("Unauthorized")
                    );

                    let query_msg = native_lockup::contract::QueryMsg::Lockup {
                        address: claimer.0.to_string(),
                    };
                    let lockup: native_lockup::storage::Lockup = ctx
                        .deps
                        .querier
                        .query_wasm_smart(lockup_contract.clone(), &query_msg)
                        .map_err(|error| error)?;

                    let count_response: native_lockup::msg::CountResponse = ctx
                        .deps
                        .querier
                        .query_wasm_smart(
                            lockup_contract,
                            &native_lockup::contract::QueryMsg::Count {},
                        )
                        .map_err(|error| error)?;

                    Ok((
                        LockupInfo {
                            owner: None,
                            amount: lockup.amount,
                            locked_since: lockup.locked_since,
                            locked_until: lockup.locked_until,
                        },
                        count_response.count,
                        String::from(""),
                    ))
                }
                "cw721" => {
                    let query_msg = cw721_lockup::contract::QueryMsg::LockupByToken {
                        collection_address: claimer.0.to_string(),
                        token_id: claimer.1.to_string(),
                    };
                    let lockup: cw721_lockup::storage::Lockup = ctx
                        .deps
                        .querier
                        .query_wasm_smart(lockup_contract.clone(), &query_msg)
                        .map_err(|error| error)?;

                    // Verify that the sender is the owner of the lockup
                    ensure_eq!(
                        lockup.owner,
                        ctx.info.sender,
                        StdError::generic_err("Unauthorized")
                    );

                    let count_response: cw721_lockup::msg::CountResponse = ctx
                        .deps
                        .querier
                        .query_wasm_smart(
                            lockup_contract,
                            &cw721_lockup::contract::QueryMsg::Count {},
                        )
                        .map_err(|error| error)?;

                    Ok((
                        LockupInfo {
                            owner: Some(lockup.owner),
                            amount: Uint128::from(1u128),
                            locked_since: lockup.locked_since,
                            locked_until: lockup.locked_until,
                        },
                        count_response.count,
                        claimer.clone().1,
                    ))
                }
                &_ => Err(StdError::generic_err("Invalid lockup contract type")),
            }?;

        let last_claim = match staking {
            Some(stake) => stake.last_claim,
            None => match lockup.locked_since > start_time {
                true => lockup.locked_since,
                false => start_time,
            },
        };

        if last_claim.plus_seconds(distribution_interval.seconds()) > ctx.env.block.time {
            return Err(StdError::generic_err("Distribution interval not reached"));
        };

        let time_factor = (Decimal::from_str(&ctx.env.block.time.seconds().to_string())?
            - Decimal::from_str(&last_claim.seconds().to_string())?)
            / Decimal::from_str(&distribution_interval.seconds().to_string())?;

        let modulated_time_factor =
            ((Decimal::from_str(&ctx.env.block.time.seconds().to_string())?
                - Decimal::from_str(&last_claim.seconds().to_string())?)
                % Decimal::from_str(&distribution_interval.seconds().to_string())?)
                / Decimal::from_str(&distribution_interval.seconds().to_string())?;

        let reward_factor = Decimal::from_str(&total_rewards.to_string())?
            / ((Decimal::from_str(&end_time.seconds().to_string())?
                - Decimal::from_str(&start_time.seconds().to_string())?)
                / Decimal::from_str(&distribution_interval.seconds().to_string())?)
            / Decimal::from_str(&count.to_string())?;

        let reward = Decimal::from((time_factor - modulated_time_factor) * reward_factor)
            * Decimal::from_str(&lockup.amount.to_string())?;

        let msg = BankMsg::Send {
            to_address: ctx.info.sender.to_string(),
            amount: vec![coin(reward.to_uint_floor().u128(), reward_denom.clone())],
        };
        let send_msg = SubMsg::new(msg);

        // Update staking information
        let new_staking = Stake {
            last_claim: ctx.env.block.time,
        };
        self.staking.save(
            ctx.deps.storage,
            (claimer.0, claimer_token_id),
            &new_staking,
        )?;

        let res = Response::new()
            .add_submessage(send_msg)
            .add_attribute("method", "claim_rewards")
            .add_attribute("contract_address", ctx.env.contract.address.to_string())
            .add_attribute("sender", ctx.info.sender.to_string())
            .add_attribute("denom", reward_denom)
            .add_attribute("amount", reward.to_string());

        Ok(res)
    }

    #[msg(exec)]
    fn withdraw_excess_balance(&self, ctx: ExecCtx) -> StdResult<Response> {
        // Admin only
        let admin = self.admin.load(ctx.deps.storage).unwrap();
        ensure_eq!(
            admin,
            ctx.info.sender,
            StdError::generic_err("Unauthorized")
        );

        // Ensure end time has been reached
        let end_time = self.end_time.load(ctx.deps.storage)?;
        ensure!(
            ctx.env.block.time > end_time,
            StdError::generic_err("End time has not been reached")
        );

        let reward_denom = self.reward_denom.load(ctx.deps.storage)?;

        let contract_address = ctx.env.contract.address.to_string();
        let contract_balance = ctx
            .deps
            .querier
            .query_balance(&contract_address, reward_denom.clone())
            .map_err(|error| error)?;

        // Send remaining balance to caller
        let msg = BankMsg::Send {
            to_address: ctx.info.sender.to_string(),
            amount: vec![coin(contract_balance.amount.u128(), reward_denom.clone())],
        };

        let send_msg = SubMsg::new(msg);

        Ok(Response::new()
            .add_submessage(send_msg)
            .add_attribute("method", "withdraw_excess_balance")
            .add_attribute("contract_address", contract_address)
            .add_attribute("sender", ctx.info.sender.to_string())
            .add_attribute("denom", reward_denom)
            .add_attribute("amount", contract_balance.amount.to_string()))
    }

    #[msg(query)]
    fn config(&self, ctx: QueryCtx) -> StdResult<ConfigResponse> {
        let admin = self.admin.load(ctx.deps.storage)?;
        let lockup_contract = self.lockup_contract.load(ctx.deps.storage)?;
        let distribution_interval = self.distribution_interval.load(ctx.deps.storage)?;
        let reward_denom = self.reward_denom.load(ctx.deps.storage)?;
        let total_rewards = self.total_rewards.load(ctx.deps.storage)?;
        let start_time = self.start_time.load(ctx.deps.storage)?;
        let end_time = self.end_time.load(ctx.deps.storage)?;

        Ok(ConfigResponse {
            admin,
            lockup_contract,
            distribution_interval,
            reward_denom,
            total_rewards,
            start_time,
            end_time,
        })
    }

    #[msg(query)]
    fn query_last_claim(&self, ctx: QueryCtx, of: (String, String)) -> StdResult<Timestamp> {
        let of_address = ctx.deps.api.addr_validate(&of.0)?;
        let claimer = (of_address, of.1);

        let staking = self.staking.load(ctx.deps.storage, claimer)?;

        Ok(staking.last_claim)
    }
}
