use cosmwasm_std::{Coin, CosmosMsg, Env};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{MsgBurn, MsgMint};

pub fn mint_to(env: Env, address: String, amount: Coin) -> CosmosMsg {
    let account: String = env.contract.address.into();

    let msg_mint: CosmosMsg = MsgMint {
        sender: account,
        amount: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            amount: amount.clone().amount.to_string(),
            denom: amount.clone().denom,
        }),
        mint_to_address: address.to_string(),
    }
    .into();

    msg_mint
}

pub fn burn(env: Env, amount: Coin) -> CosmosMsg {
    let account: String = env.contract.address.into();

    let msg_burn: CosmosMsg = MsgBurn {
        burn_from_address: account.clone(),
        sender: account,
        amount: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            amount: amount.clone().amount.to_string(),
            denom: amount.clone().denom,
        }),
    }
    .into();

    msg_burn
}
