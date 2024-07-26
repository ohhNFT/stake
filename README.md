# OhhNFT On-Chain Stake Spec

[![Rust](https://github.com/ohhNFT/sg721-stake/actions/workflows/rust.yml/badge.svg)](https://github.com/ohhNFT/sg721-stake/actions/workflows/rust.yml)

**ohhnft-stake** is a Staragaze smart contract aimed at DAOs and NFT communities to allow for token ownership-based reward distribution with both fixed & inflationary models.

```mermaid
flowchart
  subgraph Native/TF Lockup
  A{{$STRDST}} -- Deposit --> B(Vault);
  end

  subgraph SG721 Lockup
  C{{Bad Kid #1234}} -- Deposit --> D(Vault);
  end

  B -.- E;
  D -.- E;

  subgraph Stake Contract
  E([Distribute Rewards]) --> F & G;

  F(Fixed Supply) --> H([Calculate distribution]);

  G(Inflationary) --> I([Mint new tokens]);

  end
```

## Usecase

**OhhNFT On-Chain Stake** allows NFT communities to engage their users by rewarding them for holding their NFTs or tokens. The contract allows you to:

- Mint new tokenfactory tokens periodically as a proof-of-loyalty token
- Distribute incentives in native tokens such as $STARS to your holders
- Query staked NFTs per user as a metric of voting power for your NFT DAO

## Token Lockup

Both native/tokenfactory tokens and SG721 (NFT) tokens can be used with the staking contract. Depending on your needs, you can deploy either the **Native/TF Lockup** contract or the **SG721 Lockup** contract and link it to the Stake contract.

Both token lockup contracts use a 14-day lockup period by default. This can be modified by setting a custom value to the `lockup_interval` value within the instantiate message.

```js
{
  "lockup_interval": 36000,
  "collections": ["stars1..."], // Used by SG721 Lockup
  "token": "ustars" // Used by Native/TF Lockup
}
```

## Reward Distribution

### Fixed Supply

Contracts using the fixed supply model hold tokens and distribute them over a set period of time. For example, if 100 tokens are to be distributed over 10 days in 1 day intervals, 10 tokens will be available to be claimed each day. If there are 5 tokens staked in the lockup contract, each of their owners will be able to claim 2 tokens per token staked.

<table>
  <tr>
    <th>User</th>
    <th>Global</th>
  </tr>
  <tr>
  <td>
    <table>
      <li>
        <b><i>t<sub>x</sub></i></b> - time of last claim
      </li>
    </table>
  </td>
  <td>
    <table>
      <td>
        <li>
          <b><i>s</i></b> - total reward supply
        </li>
        <li>
          <b><i>i</i></b> - distribution interval
        </li>
        <li>
          <b><i>n<sub>t</sub></i></b> - total tokens locked
        </li>
      </td>
      <td>
      <li>
        <b><i>t</i></b> - current time
      </li>
      <li>
        <b><i>t<sub>a</i></b> - start time
      </li>
      <li>
        <b><i>t<sub>b</sub></i></b> - end time
      </li>
    </td>
    </table>
  </td>
  </tr>
</table>

$$R = \left(\frac{t-t_x}{i}-\frac{(t-t_x)\mod i}{i}\right)\cdot\left(s\div\frac{t_b-t_a}{i}\div n_t\right)$$

### Inflationary Model

An inflationary model is not supported if your distributed rewards are in $STARS. This model requires the instantiator to be the admin of a tokenfactory token.

When using an inflationary model, the contract will mint a set amount of tokens per interval, splitting it evenly between tokens staked.

<table>
  <tr>
    <th>User</th>
    <th>Global</th>
  </tr>
  <tr>
  <td>
    <table>
      <li>
        <b><i>t<sub>x</sub></i></b> - time of last claim
      </li>
    </table>
  </td>
  <td>
    <table>
      <td>
        <li>
          <b><i>s</i></b> - reward per interval
        </li>
        <li>
          <b><i>i</i></b> - distribution interval
        </li>
        <li>
          <b><i>n<sub>t</sub></i></b> - total tokens locked
        </li>
      </td>
      <td>
      <li>
        <b><i>t</i></b> - current time
      </li>
      <li>
        <b><i>t<sub>a</i></b> - start time
      </li>
    </td>
    </table>
  </td>
  </tr>
</table>

$$R = \frac{s}{n_t}\cdot\left(\frac{t-t_x}{i}-\frac{(t-t_x)\mod i}{i}\right)$$

## Claiming Rewards

Rewards must be claimed manually by the user. For SG721-based setups, **users will have to claim rewards for each NFT manually**. This can be facilitated by the frontend by grouping transactions for each of the tokens together.

## Storage

### Native/TF Lockup

Key is of type `Addr`

```rust
{
  amount: Uint128,
  locked_until: Timestamp
}
```

### CW721 Lockup

```rust
{
  owner: Addr,
  collection_address: Addr,
  token_id: String,
  locked_until: Timestamp
}
```

### Stake Contract

Key is of type `Addr` or `(Addr, u64)`, depending on the lockup contract used.

```rust
{
  last_claim: Timestamp
}
```
