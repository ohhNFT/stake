# Upload Stake721

We are proposing to upload the code for the Stake721 suite of hard staking contracts for OhhNFT.

The source code is available at: https://github.com/OhhNFT/stake721

### Contracts

#### CW721 Lockup

This contract will hold CW721/SG721 NFTs for a set lockup period then allow withdrawals. NFTs can be deposited by simply sending them to the contract.

####

### Testnet deployment

- Code ID: `4249`
- Contract: `stars1k2pwhlmpl7elmf2kvpcjhjz79ushuqcvsv23dgp4v3uxlrxh0e6qnte4u0`

### SHA256 checksum

```
84764c3a19979f249d885f9390ca88e0bcbec4927b48859d7e31fd19afcf2faa  raffles.wasm
```

### Verify code

```
starsd  q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum
```
