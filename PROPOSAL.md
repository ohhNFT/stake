# Upload Stake721

We are proposing to upload the code for the Stake721 suite of hard staking contracts for OhhNFT.

The source code is available at: https://github.com/OhhNFT/stake721

## Contracts

### CW721 Lockup

This contract will hold CW721/SG721 NFTs for a set lockup period then allow withdrawals. NFTs can be deposited by simply sending them to the contract.

### Native Lockup

This contract will hold native, IBC and TokenFactory tokens for a set lockup period. Tokens can be deposited by calling `Deposit {}` with funds.

### Fixed Stake

This contract will distribute rewards over set intervals and over a set period of time to all token holders when `ClaimRewards { of }` is called.

## Claiming rewards

### CW721-based Stake

Stake contracts linked to a CW721 Lockup contract require users to provide the collection address and token ID when claiming rewards, as so:

```json
{
  "claim_rewards": {
    "of": ["stars1...", "1"]
  }
}
```

### Native-based Stake

Stake contracts linked to a Native Lockup contract only require users to provide the first of the two values in `of` and to leave the second value a blank string, as so:

```json
{
  "claim_rewards": {
    "of": ["stars1...", ""]
  }
}
```

## Testnet deployments

### Native Lockup

- Code ID: `4402`
- Contract: `stars1nrmmqrghd77t3vd52hnl4rphvqxaqmx0vh8yl6hrrk2n4vevvkms4jmye5`

### Fixed Stake

- Code ID: `4403`
- Contract: `stars1xz9r5f7858vjya6fzk68qmdsx4h70gjaqhe02l88t8ats2y20shqfsc65p`

### SHA256 checksums

```
a37995020e9af393f288ea18fc758d4720ca595f404f492de63ffc553be0bdea  cw721_lockup.wasm
61434b3c799a4825013afa9fdae60b376955f952864fe47e25f01799024d08f4  fixed_stake.wasm
dbc5ab5a58be5c09c8393dd667fa5a6f66f31b7366964912608da1895fb87894  native_lockup.wasm
```
