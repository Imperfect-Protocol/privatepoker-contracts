# privatepoker-contracts

Arbitrum Stylus contracts for Private Poker.

This workspace contains the Private Poker diamond, lobby/table/hand/spectate facets, signalling contract, chips/cashier/test USDC contracts, and shuffle/unmasking verification contracts.

## Required Environment

Before running deployment scripts, load your private environment file:

```sh
. ~/some-hidden-location/privatepoker-devenv.sh
```

That file should export:

```sh
export PP_HOME=/path/to/private/env/storage
export PP_ENV=nitro-devnode
export PP_PRIVATE_KEY=0x...
export PP_OWNER=0x...
export RPC_URL=http://localhost:8547
```

For the local Nitro DevNode test deploy, the current development key is:

```sh
export PP_PRIVATE_KEY=0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659
export PP_OWNER=$(cast wallet address "$PP_PRIVATE_KEY")
export RPC_URL=http://localhost:8547
```

Generated deployment env files are written under:

```sh
$PP_HOME/.privatepoker.$PP_ENV.$CHAIN_ID.env
$PP_HOME/.privatepoker.$PP_ENV.test-usdc.$CHAIN_ID.env
```

The scripts load those generated env files automatically when `PP_HOME` and `PP_ENV` are set.

## Check Contract Builds

From `/privatepoker-contracts`:

```sh
./scripts/check-all-contracts.sh
```

This runs `cargo stylus check` for every contract and prints a size table at the end.

To check a single contract:

```sh
./scripts/check-contract.sh signal
```

## Deploy Test USDC

Deploy fake USDC for Nitro DevNode or Sepolia:

```sh
./scripts/deploy-privatepoker-test-usdc.sh
```

This writes:

```sh
export PP_TEST_USDC=...
export PP_USDC=...
```

to:

```sh
$PP_HOME/.privatepoker.$PP_ENV.test-usdc.$CHAIN_ID.env
```

To intentionally overwrite an existing TEST_USDC env file:

```sh
PP_FORCE_TEST_USDC_DEPLOY=1 ./scripts/deploy-privatepoker-test-usdc.sh
```

## Deploy Private Poker Core

Deploy the core contracts:

```sh
./scripts/deploy-privatepoker.sh
```

This deploys:

- `PP_LOBBY`: diamond address
- `PP_LOBBY_FACET`
- `PP_TABLE_FACET`
- `PP_HAND_FACET`
- `PP_SPECTATE_FACET`
- `PP_CHIPS`
- `PP_CASHIER`
- `PP_SIGNAL`
- `PP_VERIFY_SHUFFLE`
- `PP_VERIFY_UNMASKING`

It also wires:

- Chips cashier address
- Chips lobby address
- Lobby chip token address

The generated env file is:

```sh
$PP_HOME/.privatepoker.$PP_ENV.$CHAIN_ID.env
```

To intentionally overwrite an existing core deployment env file:

```sh
PP_FORCE_PRIVATEPOKER_DEPLOY=1 ./scripts/deploy-privatepoker.sh
```

## Create Development Lobby

After deploying core contracts, create lobby `1`:

```sh
./scripts/cast-send.sh "$PP_LOBBY" 'addLobby(uint256,uint256,uint256,string)' 1 1 1 "Texas Holdem"
```

## Fund Test Accounts

Mint TEST_USDC to all standard Anvil accounts:

```sh
./scripts/fund-test-usdc-to-test-accounts.sh 100000
```

`100000` is cents, so this mints `1000.00` TEST_USDC to each account.

Mint TEST_USDC to one address:

```sh
./scripts/fund-test-usdc.sh 100000 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
PP_PRIVATE_KEY=0x ./scripts/deposit-test-usdc.sh 50000
```

If the Nitro DevNode accounts do not already have ETH, fund the first three player accounts from the funded dev owner key:

```sh
cast send --private-key "$PP_PRIVATE_KEY" --rpc-url "$RPC_URL" --value 1ether 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
cast send --private-key "$PP_PRIVATE_KEY" --rpc-url "$RPC_URL" --value 1ether 0x70997970C51812dc3A010C7d01b50e0d17dc79C8
cast send --private-key "$PP_PRIVATE_KEY" --rpc-url "$RPC_URL" --value 1ether 0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC
```

## Local DevNode Workflow

```sh
cd /privatepoker-contracts

. ~/some-hidden-location/privatepoker-devenv.sh

./scripts/check-all-contracts.sh
./scripts/deploy-privatepoker-test-usdc.sh
./scripts/deploy-privatepoker.sh
./scripts/cast-send.sh "$PP_LOBBY" 'addLobby(uint256,uint256,uint256,string)' 1 1 1 "Texas Holdem"
./scripts/fund-test-usdc-to-test-accounts.sh 100000
```

Then run the three `crum_p2p_bot` terminals documented in:

```sh
/crumble/apps/crum_p2p_bot/README.md
```

