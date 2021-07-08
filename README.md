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

For more details see [substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

## Build

1. `docker build . -t eu.gcr.io/satoshipay-206315/pendulum/node:latest`
2. `docker push eu.gcr.io/satoshipay-206315/pendulum/node:latest`

## Deploy

1. Open the [Google Cloud Run console](https://console.cloud.google.com/run)
2. Open the `pendulum-demo-node` service
3. Choose `Edit and deploy new version`
4. Change the `Container image URL`, pick the latest container image
5. Deploy

