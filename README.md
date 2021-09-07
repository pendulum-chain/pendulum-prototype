<div background="red">
  <h2 align="center">🚧 Under construction 🚧</h2>
  <h3 align="center">This is an early prototype right now. Do not try to run this in production!</h3>
</div>
<br>

# Pendulum Chain: Second Layer Network by SatoshiPay

Second layer network node and the pallets (modules) it requires.

# Pendulum Chain Prototype

The Pendulum prototype is meant to be run in a single Node, which acts as the bridge between Stellar and the Pendulum network itself. The network is composed of that same node also.

Substrate already provides a lot of the mechanisms needed for a full working network with several nodes, validators, etc. So this proof of concept is only focused on the bridging.

## Running the n**ode**

The repository is hosted in [pendulum-chain/pendulum-prototype](https://github.com/pendulum-chain/pendulum-prototype), and it is based on [substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

## **Setup locally**

1. [Complete rust setup](https://github.com/substrate-developer-hub/substrate-node-template/blob/master/docs/rust-setup.md) on your machine.
2. `git clone git@github.com:pendulum-chain/pendulum-prototype.git`
3. `cd pendulum-prototype`
4. `cargo run --release -- --dev --tmp`

The last step takes a while to compile the first time, depending on your OS and hardware.

It can take up to 10 minutes in some cases. Once running, you'll see a log with blocks that are already being finalized every some seconds. You should see something like this:

## **Build docker images for deployment**

1. `docker build . -t eu.gcr.io/satoshipay-206315/pendulum/node:latest`
2. `docker push eu.gcr.io/satoshipay-206315/pendulum/node:latest`

# Running the Pendulum UI

The Pendulum UI is hosted in [pendulum-chain/pendulum-ui](https://github.com/pendulum-chain/pendulum-ui), and it's a fork of the [polkadot{.js} apps](https://github.com/polkadot-js/apps) with some tweaks specific to our network.

1. [Install yarn](https://classic.yarnpkg.com/en/docs)
2. `git clone git@github.com:pendulum-chain/pendulum-ui.git`
3. `cd pendulum-prototype`
4. `yarn install`
5. `yarn run start`
6. Open `localhost:3000`

The UI and the Node are already configured to run in known ports, so you don't need to do anything in particular to connect them. You should see something like this:

---

## Known issues

1. The chain extension used for the AMM allow any arbitrary transfer of funds. This should use an ERC20-like mechanism to secure it.
2. Slow UI explorer. The UI is based in polkadot{.js} apps, which is very heavy because it is an all-purpose application, supporting any kind of polkadot parachains. We have slow loading times (specially the first load can take up to 20 seconds or more!) because we need to load a lot of unnecessary code. This could be solved by using a better cache, or alternatively, building a simple React application that makes direct use of the polkadot{.js} API.
3. Deploy the node to a permanent hosting. Right now and because of the specifications of our system, the node will periodically restart, so the network will go back to the genesis after a period of time.
4. Even though the node supports all kind of assets mirrored form Stellar, the UI still has to know a list of known assets beforehand, to be able to show the balances for that assets. This list can be as long as we want, so in practice we could support the most used assets. This will change to accommodate arbitrary assets in future.
5. We still don't have a manner to list the balances of "all accounts", but instead we need to add the accounts we want to check by hand. This is good from the user perspective, but inconvenient for testing.
6. We have a limitation on the amount of signers an escrow account can have (20). Thus, we can't have more than 20 bridge nodes for a single escrow account.
