# SP1 Contracts

This repository contains the smart contracts for verifying [SP1](https://github.com/succinctlabs/sp1) EVM proofs.

## Installation

> [!WARNING]
>
> [Foundry](https://github.com/foundry-rs/foundry) installs the latest release version initially, but subsequent `forge update` commands will use the `main` branch. This branch is the development branch and should be avoided in favor of tagged releases. The release process matches a specific SP1 version.

To install the latest release version:

```bash
forge install succinctlabs/sp1-contracts
```

Add `@sp1-contracts/=lib/sp1-contracts/contracts/src/` in `remappings.txt.`

### Usage

Once installed, you can use the contracts in the library by importing them:

```solidity
pragma solidity ^0.8.25;

import {SP1Verifier} from "@sp1-contracts/SP1Verifier.sol";

contract MyContract is SP1Verifier {
}
```

### Deployments

To deploy the contracts, ensure your [.env](./contracts/.env.example) file is configured with all the chains you want to deploy to.

Then you can use the `forge script` command and specify the specific contract you want to deploy. For example, to deploy the SP1 Verifier Gateway you can run:

```bash
FOUNDRY_PROFILE=deploy forge script ./script/deploy/SP1VerifierGateway.s.sol:SP1VerifierGatewayScript --private-key $PRIVATE_KEY --verify --verifier etherscan --multi --broadcast
```

To deploy a specific SP1 Verifier version and add it to the gateway, run:

```bash
FOUNDRY_PROFILE=deploy forge script ./script/deploy/v1.0.8-testnet/SP1Verifier.s.sol:SP1VerifierScript --private-key $PRIVATE_KEY --verify --verifier etherscan --multi --broadcast
```

Change `v1.0.8-testnet` to the desired version to add.

To re-verify already existing deployments, remove the `--broadcast` flag.

## For Developers: Integrate SP1 Contracts

This repository contains the EVM contracts for verifying SP1 PLONK EVM proofs.

You can find more details on the contracts in the [`contracts`](./contracts/README.md) directory.

Note: you should ensure that all the contracts are on Solidity version `0.8.20`.

## For Contributors

To update the SP1 contracts, please refer to the [`update`](./UPDATE_CONTRACTS.md) file.
