<div background="red">
  <h2 align="center">🚧 Under construction 🚧</h2>
  <h3 align="center">This is an early prototype right now. Do not try to run this in production!</h3>
</div>
<br>

# Pendulum Chain: Second Layer Network by SatoshiPay

Second layer network node and the pallets (modules) it requires.

Based on Substrate. Repository based on [substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

# Pendulum Chain Prototype

The Pendulum prototype is meant to be run on a single node, which acts as the bridge between Stellar and the Pendulum network itself. The network is composed of that same node also.

Substrate already provides a lot of the mechanisms needed for a full working network with several nodes, validators, etc. So this proof of concept is only focused on the bridging.

To be able to test the node and the UI, you can use our deployed version:

- [https://prototype-ui.pendulumchain.org/?rpc=wss%3A%2F%2Flatest---pendulum-demo-node-5agyjkoilq-uc.a.run.app%3A443#/explorer](https://prototype-ui.pendulumchain.org/?rpc=wss%3A%2F%2Flatest---pendulum-demo-node-5agyjkoilq-uc.a.run.app%3A443#/explorer)

Be aware that the website might take 20 seconds to load.

*Note:  pointing to the node address from the regular polkadot js explorer won't work, you need to use our explorer.*

Follow the steps in the section [Running the demo](#running-the-demo) to test the Stellar Pendulum bridge and a smart contract AMM yourself.

Alternatively, we show you how you can [deploy your own nodes](#running-the-node) and [explorer](#running-the-pendulum-ui) to tweak them as you want ;)

# Running the demo

This description applies to both

- Your [own node](#running-the-node) and [Pendulum UI](#running-the-pendulum-ui) as described in this documentation
- Our [deployed node and UI](https://prototype-ui.pendulumchain.org/?rpc=wss%3A%2F%2Flatest---pendulum-demo-node-5agyjkoilq-uc.a.run.app%3A443#/explorer)

## **Prerequisites**

- We recommend that you use [Solar](https://solarwallet.io/) to interact with the Stellar side of your accounts
- Currently you can only bridge assets that are supported by our escrow account (more precisely, the escrow account requires trustlines for assets that are to be bridged between Stellar and Pendulum). Our escrow account has some trustlines already set for testing. You can check them on [https://stellar.expert/explorer/testnet/account/GALXBW3TNM7QGHTSQENJA2YJGGHLO3TP7Y7RLKWPZIY4CUHNJ3TDMFON](https://stellar.expert/explorer/testnet/account/GALXBW3TNM7QGHTSQENJA2YJGGHLO3TP7Y7RLKWPZIY4CUHNJ3TDMFON).

    The ones that we will use in this documentation are: 

    - `USDC`, issuer: `GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC`
    - `EUR`, issuer: `GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC`
- You need to have an existing and funded Stellar testnet account. You can easily [create](https://docs.solarwallet.io/guide/04-account-management.html#create-stellar-account) an account in Solar and fund it with the [testnet friendbot](https://docs.solarwallet.io/guide/09-testnet.html#friendbot).
- Set the trustlines for the assets you want to bridge for your Stellar account ([step by step guide](https://docs.solarwallet.io/guide/07-asset-management.html#add-custom-assets) on how to set trustlines for custom assets in Solar).
- Fund your account with those assets. For this purpose use the asset issuer as a faucet. Here we describe how to mint tokens using [Stellar Laboratory](https://laboratory.stellar.org/).
    - Go to the tab **`Build Transaction`** and paste the issuer of the above two assets as a **`Source Account`**. Then click on the button `**Fetch next sequence number for account starting with ...**`
    - In the dropdown at the bottom labeled `**Operation Type**` choose **`Payment`**. As **`Destination`** enter the [account Id of your Stellar account](https://docs.solarwallet.io/guide/05-payments.html#receive-payment). Then select **`Alphanumeric 4`** in the field **`Asset`** and enter the `**Asset Code**` (e.g., `USDC`) and **`Issuer Account ID`** (`GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC`) of either one of the assets you want to bridge. Enter any amount, e.g., `1000`. Optionally click on "+ Add Operation" and add another payment operation for another asset.
    - Finally click on **`Sign in Transaction Signer`**, which leads to another page. In the field **`Add Signer`** add the secret key of the asset issuer: `SA4OOLVVZV2W7XAKFXUEKLMQ6Y2W5JBENHO5LP6W6BCPBU3WUZ5EBT7K`. Afterwards click on **`Submit in Transaction Submitter`**.
    - Click on **`Submit Transaction`**. After a few seconds a green message "**Transaction submitted!**" will be displayed.

*Note: only those assets listed above are currently supported by the escrow account and can be bridged to Pendulum prototype.*

## Testing

### **Add an account**

1. In our UI click on **`Accounts`** at the top from the menu bar to see the current accounts in use.
2. Press the **`+ Add Account`**button, to add your own Stellar account. After this, you should see your account listed below, with your current balances in Pendulum. You only need to add your account once, the information is securely stored in the local storage.
    1. Select `**Stellar**` as the seed type
    2. Input your [Stellar account secret key](https://docs.solarwallet.io/guide/04-account-management.html#export-secret-key)
    3. Tick the checkbox **`I have saved my Stellar secret key safely`** and click on **`Next`**
    4. Give the account a name and password, click **`Next`** and then **`Save`** and you're done!

### **Add PEN tokens**

For some operations on Pendulum (e.g. withdrawal and AMM swaps), you need PEN tokens to pay the gas fees. To get some PEN on your Pendulum account, you must add the PEN faucet and transfer PEN to your list of accounts:

1. Follow the procedure to [add a new account](#add-an-account) and use the secret key: `SCV7RZN5XYYMMVSWYCR4XUMB76FFMKKKNHP63UTZQKVM4STWSCIRLWFJ`
2. Once the faucet is added, transfer 3000 PEN tokens from the faucet to your account by clicking on the "send" button next to the faucet account in your account view. 
3. In the account view you will then see that your account has some PEN tokens.


### **Deposit funds from stellar**

1. From your Stellar wallet of preference (we recommend using [Solar Wallet](https://solarwallet.io/)), transfer the desired amount of either one of the assets supported by our bridge to the known escrow account: `GALXBW3TNM7QGHTSQENJA2YJGGHLO3TP7Y7RLKWPZIY4CUHNJ3TDMFON` 

    Wait until the transaction is successfully completed:

2. After a short time, you should see your balance incremented in Pendulum.

    Before

    After

### **Withdraw funds to stellar**

1. Ensure you have PEN tokens on the account (see "[Add PEN tokens](#add-pen-tokens)" to get some)
2. Go to **`Developer > Extrinsic's page`**
3. Select the account to be withdrawn from
4. In the field **`submit the following extrinsic`** select **`stellarBridge`** from the first and **`widthrawToStellar`**(...) from the second dropdown
5. Specify the asset code(for ex. USDC, EUR etc), issuer (e.g., `GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC`) and amount
6. Click on **`Submit transaction`**.
7. Enter the password of your account, then click on **`Sign and submit`**. 
8. If everything goes ok, you should see a green sign. After some moments you should see your balance incremented on Stellar.


9. If there's an error, you can double-check that you have enough funds of the select token, and that the asset code and issuer are correctly written.

## Automated Market Maker

One of the main features in Pendulum chain is to have smart contracts, specially to be able to have an Automated Market Maker. For that reason, we've implemented a simple version of an AMM that users can use for testing.

### **Get the contract file**

For using the AMM, you will need the `pendulum_amm.contract` file, you can get it from our releases or compile one by yourself.

a) **[Get the precompiled version from our releases (recommended)](https://github.com/pendulum-chain/pendulum-amm/releases)** 

b) Compile the AMM

1. Follow the steps in [https://paritytech.github.io/ink-docs/getting-started/setup](https://paritytech.github.io/ink-docs/getting-started/setup) to install all required tools for compiling and running the smart contract
2. Clone the repository of the AMM ([Pendulum AMM Github repository](https://github.com/pendulum-chain/pendulum-amm))
3. Enter the root directory of the cloned repository and run `cargo +nightly contract build`
4. The result is a  `pendulum_amm.contract` file that can be used to deploy your contract to the pendulum chain. You can find it in `./target/ink/pendulum_amm.contract`.

### **Deploy the AMM**

1. Go to  `**Developer > Contracts**`
2. Press the **`+ Upload & deploy code`** button
3. Choose your account as the deployment account. It is important that your account has sufficient PEN tokens available (see "[Add PEN tokens](#add-pen-tokens)" to get some).
4. Click on the file drop-zone, select the `pendulum_amm.contract` file you compiled or downloaded [as described above](#get-the-contract-file), choose a code bundle name and click **`Next`**.
5. For the construction of the smart contract, enter the corresponding values in the respective text fields:
    1. `assetCode0`: The asset code of the first asset (for example, `USDC`)
    2. `issuer0`: this asset's issuer account Id (e.g., `GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC`) 
    3. `assetCode1`: The asset code of the other asset (for example, `EUR`)
    4. `issuer1`: this asset's issuer account Id as you would use it on Stellar (e.g., `GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC`)
6. Enter `1000` units in the  `**endowment**` text field below the constructor values. You need to make sure your account has enough PEN tokens (in this case at least a bit more than `1000` – see "[Add PEN tokens](#add-pen-tokens)" to get some).
7. Click on the **`Deploy`** button, enter the password of your account and then click on the **`Sign and Submit`** button to deploy the smart contract. If successful, you should see something like this:

   
## Using the AMM

After deploying the AMM contract go to `**Developer > Contracts**`. In the list below "contracts" you should find a contract called **`PENDULUM AMM`**or any custom name you specified when deploying the contract. Click on the arrow next to **`Messages (13)`** to expand the available functions

### **Prerequisites**

- Make sure that the account you want to use has enough funds for the operations you want to do. For the example described here, make sure you have at least 1 unit of `EUR` and `USDC`.  You can receive tokens by following the steps described [in the section about the bridge]().
- Ensure you have PEN tokens on the account (see "[Add PEN tokens](#add-pen-tokens)" to get some).

**Hint**: In the following, when you are told to "Click on **`Execute`** and `**Sign and Submit**`". Before doing so, you can always turn on the small switch shown in the screenshot. This setting changes the contract call so that it does not actually execute the transaction but only reads the result, similar to a dry-run. This way you can check if the contract call would be successful or if it yields an error before actually submitting it. If the call result contains something like `{ Ok: ... }` your contract call is going to be successful, so you can uncheck the switch again and execute the transaction on the network. If the call result contains an error, you should re-check your input values. 


### **Deposit**

This operation adds liquidity to the AMM. The current reserves in the AMM can be determined via the `getReserves` function of the AMM. This returns an array that contains the amount of the first asset and the amount of the second asset. For technical reasons this function returns the pico-units instead of the units of the of the amounts. Divide the pico-units by 1 trillion. They are 0 initially. 

This requires that your account has sufficient amounts of *both* assets. 

1. Click on the `**exec**` button next to **`depositAsset1`** or **`depositAsset2`**
2. Select your account in the field **`call from account`**.
3. Enter the desired value in the **`amount`** text field.
4. Click on **`Execute`**, enter your account's password and click on `**Sign and Submit`.**

### **Swap**

This requires that the AMM already has some reserves. The current reserves in the AMM can be determined via the `getReserves` function. [Execute an AMM deposit](#deposit) first to create and add reserves.

1. Click on the **`exec`** button next to `**swapAsset1ForAsset2**` or `**swapAsset2ForAsset1**`
2. Enter the account that is to execute the swap in the **`call from account`** field.
3. Enter an amount in the `**amountToReceive**` text field. This amount must be below the amount of the reserve of the asset to be received.
4. Click on **`Execute`**, enter the password of the account executing the swap and click on `**Sign and Submit**`
5. After a successful swap you will see that the balances of the assets of the swapping account and the reserves of the AMM changed accordingly.

### **Withdraw**

If your account provided liquidity to the AMM [using a deposit](#deposit), then it can withdraw liquidity from the AMM at a later point in time.

1. Check how many liquidity tokens you have by clicking on the `**Read**` button next to the function `**lpBalanceOf`** . Specify your account in the field **`owner`** and click the `**Read**` button.
2. Copy the value of the call result that is shown below **`Call results`** and remember the unit that is shown next to it. Close the dialog.
3. Click on the `**exec**` button next to **`withdraw`**.
4. Enter your account in the **`field call from account`** and in the field **`to`**.
5. Paste the value you copied (or any smaller amount) into the `**amount**` text field. For example, if your call result balance from step 3c is `99.9999 mUnit` select `milli` in the **`Unit`** dropdown next to the amount text field and then enter `99.9999` as the amount.
6. Click on `**Execute**` enter your account's password and click on `**Sign and Submit**`.
7. After a successful withdrawal you will see that the balances of the assets of your account increase and the reserves of the AMM decrease accordingly.

**Troubleshooting**

- The AMM is governed by a mathematical function. The price of each asset is determined by the total amount of both assets, i.e., by the ratio of the amount of its reserves. If you want to test it like a real case scenario, be sure you don't use values high enough that make the relation of both assets too unbalanced. For example, if you deposited 5 units of each asset when swapping it's better to use a value such as `0.01` instead of `3`.
- Make sure the balances in the source account are sufficient for depositing, and the reserves in the AMM are sufficient for swapping.
- Pay attention to the correct units. In every numeric input, you'll find a `**units**` dropdown. If you want to use a value such as `0.00001` units, you can also use `10` micro-units, and so on.
- If you deploy the contract and transfer less than what is specified above for the `**endowement**`, then you will probably experience, because the contract won't have enough balance to operate correctly.
- If you see an error like this: `NotCallable` , for some reason, the contract is no more available, so you might need to redeploy the contract.


# Running the node

The repository is hosted in [pendulum-chain/pendulum-prototype](https://github.com/pendulum-chain/pendulum-prototype), and it is based on [substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template).

## Setup locally

1. [Complete rust setup](https://github.com/substrate-developer-hub/substrate-node-template/blob/master/doc/rust-setup.md) on your machine.
2. `git clone git@github.com:pendulum-chain/pendulum-prototype.git`
3. `cd pendulum-prototype`
4. `cargo run --release -- --dev --tmp`

The last step takes a while to compile the first time, depending on your OS and hardware.

It can take up to 10 minutes in some cases. Once running, you'll see a log with blocks that are already being finalized every some seconds. You should see something like this:



## Build docker images for deployment

1. `docker build . -t eu.gcr.io/satoshipay-206315/pendulum/node:latest`
2. `docker push eu.gcr.io/satoshipay-206315/pendulum/node:latest`

# Running the Pendulum UI

The Pendulum UI is hosted in [pendulum-chain/pendulum-ui](https://github.com/pendulum-chain/pendulum-ui), and it's a fork of the [polkadot{.js} apps](https://github.com/polkadot-js/apps) with some tweaks specific to our network.

1. [Install yarn](https://classic.yarnpkg.com/en/docs)
2. `git clone git@github.com:pendulum-chain/pendulum-ui.git`
3. `cd pendulum-ui`
4. `yarn install`
5. `yarn run start`
6. Open `localhost:3000`

The UI and the Node are already configured to run in known ports, so you don't need to do anything in particular to connect them. You should see something like this:

---

## Known issues

- The chain extension used for the AMM allows any arbitrary transfer of funds. This should use an ERC20-like mechanism to secure it.
- Slow UI explorer. The UI is based in polkadot{.js} apps, which is very heavy because it is an all-purpose application, supporting any kind of polkadot parachains. We have slow loading times (especially the first load can take up to 20 seconds or more!) because we need to load a lot of unnecessary code. This could be solved by using a better cache, or alternatively, building a simple React application that makes direct use of the polkadot{.js} API.
- Deploy the node to permanent hosting. Right now and because of the specifications of our system, the node will periodically restart, so the network will go back to the genesis after a period of time.
- Even though the node supports all kinds of assets mirrored from Stellar, the UI still has to know a list of known assets beforehand, to be able to show the balances for that assets. This list can be as long as we want, so in practice we could support the most used assets. This will change to accommodate arbitrary assets in the future.
- We still don't have a manner to list the balances of "all accounts", but instead we need to add the accounts we want to check by hand. This is good from the user perspective, but inconvenient for testing.
- We have a limitation on the amount of signers an escrow account can have (20). Thus, we can't have more than 20 bridge nodes for a single escrow account.
