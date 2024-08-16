# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2](https://github.com/ethereum-optimism/kona/compare/kona-primitives-v0.0.1...kona-primitives-v0.0.2) - 2024-08-16

### Fixed
- fix superchain registry + primitives versions ([#425](https://github.com/ethereum-optimism/kona/pull/425))
- *(derive)* Granite Hardfork Support ([#420](https://github.com/ethereum-optimism/kona/pull/420))
- *(deps)* Bump Alloy Dependencies ([#409](https://github.com/ethereum-optimism/kona/pull/409))
- pin two dependencies due to upstream semver issues ([#391](https://github.com/ethereum-optimism/kona/pull/391))

### Other
- Bump `superchain-registry` version ([#306](https://github.com/ethereum-optimism/kona/pull/306))

## [0.0.1](https://github.com/ethereum-optimism/kona/releases/tag/kona-primitives-v0.0.1) - 2024-06-22

### Added
- *(kona-derive)* Towards Derivation ([#243](https://github.com/ethereum-optimism/kona/pull/243))
- *(ci)* Dependabot config ([#236](https://github.com/ethereum-optimism/kona/pull/236))
- *(client)* `StatelessL2BlockExecutor` ([#210](https://github.com/ethereum-optimism/kona/pull/210))
- *(primitives)* move attributes into primitives ([#163](https://github.com/ethereum-optimism/kona/pull/163))
- *(plasma)* Implements Plasma Support for kona derive ([#152](https://github.com/ethereum-optimism/kona/pull/152))
- *(primitives)* kona-derive type refactor ([#135](https://github.com/ethereum-optimism/kona/pull/135))

### Fixed
- use 2718 encoding ([#231](https://github.com/ethereum-optimism/kona/pull/231))
- Strong Error Typing ([#187](https://github.com/ethereum-optimism/kona/pull/187))
- *(primitives)* use decode_2718() to gracefully handle the tx type ([#182](https://github.com/ethereum-optimism/kona/pull/182))
- *(ci)* Release plz ([#145](https://github.com/ethereum-optimism/kona/pull/145))
- *(workspace)* Release plz ([#138](https://github.com/ethereum-optimism/kona/pull/138))

### Other
- version dependencies ([#296](https://github.com/ethereum-optimism/kona/pull/296))
- re-export input types ([#279](https://github.com/ethereum-optimism/kona/pull/279))
- *(deps)* fast forward op alloy dep ([#267](https://github.com/ethereum-optimism/kona/pull/267))
- use alloy withdrawal type ([#213](https://github.com/ethereum-optimism/kona/pull/213))
