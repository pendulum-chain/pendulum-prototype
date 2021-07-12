<div background="red">
  <h2 align="center">🚧 Under construction 🚧</h2>
  <h3 align="center">This is an early prototype right now. Do not try to run this in production!</h3>
</div>
<br>

# Pendulum Chain: Second Layer Network by SatoshiPay

Second layer network node and the pallets (modules) it requires.

Based on Substrate. Repository based on [substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

## Setup

1. [Complete rust setup](https://github.com/substrate-developer-hub/substrate-node-template/blob/master/docs/rust-setup.md) on your machine.
2. Clone this repository.
3. Build and run: `cargo run --release -- --dev --tmp`


## Run explorer

To be able to use the Polkadot JS explorer properly, some types need to be added to the UI via

1. Go to https://nodleprotocol.io/?rpc=ws://127.0.0.1:9944.
2. Click on Settings -> Developer
3. Add the following types

```json
{
  "CurrencyId": {
    "_enum": [
      "Native",
      "USDC"
    ]
  },
  "CurrencyIdOf": "CurrencyId",
  "Currency": "CurrencyId",
  "BalanceOf": "Balance",
  "Amount": "i128",
  "AmountOf": "Amount",
  "DepositPayload": {
    "_struct": {
      "currency_id": "CurrencyId",
      "amount": "Balance",
      "destination": "AccountId",
      "signed_by": "AccountId"
    }
  }
}```


---

For more details see [substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).
