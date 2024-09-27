# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3](https://github.com/anton-rs/kona/compare/kona-primitives-v0.0.2...kona-primitives-v0.0.3) - 2024-09-27

### Added

- Remove L2 Execution Payload ([#542](https://github.com/anton-rs/kona/pull/542))
- *(derive)* Typed error handling ([#540](https://github.com/anton-rs/kona/pull/540))
- *(primitives)* Remove Attributes ([#529](https://github.com/anton-rs/kona/pull/529))
- large dependency update ([#528](https://github.com/anton-rs/kona/pull/528))
- *(primitives)* reuse op-alloy-protocol channel and block types ([#499](https://github.com/anton-rs/kona/pull/499))

### Fixed

- *(workspace)* hoist and fix lints ([#577](https://github.com/anton-rs/kona/pull/577))
- *(primitives)* use consensus hardforks ([#497](https://github.com/anton-rs/kona/pull/497))
- *(primitives)* re-use op-alloy frame type ([#492](https://github.com/anton-rs/kona/pull/492))

### Other

- rm depo and import op::depo ([#518](https://github.com/anton-rs/kona/pull/518))
- *(primitives)* rm RawTransaction ([#505](https://github.com/anton-rs/kona/pull/505))

## [0.0.2](https://github.com/anton-rs/kona/compare/kona-primitives-v0.0.1...kona-primitives-v0.0.2) - 2024-09-04

### Added
- update superchain registry deps ([#463](https://github.com/anton-rs/kona/pull/463))
- *(primitives)* `serde` for `L1BlockInfoTx` ([#460](https://github.com/anton-rs/kona/pull/460))

### Fixed
- *(examples)* Revm Features ([#482](https://github.com/anton-rs/kona/pull/482))
- *(workspace)* Use published `revm` version ([#459](https://github.com/anton-rs/kona/pull/459))
- downgrade for release plz ([#458](https://github.com/anton-rs/kona/pull/458))
- *(workspace)* Add Unused Dependency Lint ([#453](https://github.com/anton-rs/kona/pull/453))
- fix superchain registry + primitives versions ([#425](https://github.com/anton-rs/kona/pull/425))
- *(derive)* Granite Hardfork Support ([#420](https://github.com/anton-rs/kona/pull/420))
- *(deps)* Bump Alloy Dependencies ([#409](https://github.com/anton-rs/kona/pull/409))
- pin two dependencies due to upstream semver issues ([#391](https://github.com/anton-rs/kona/pull/391))

### Other
- *(workspace)* Alloy Version Bumps ([#467](https://github.com/anton-rs/kona/pull/467))
- *(workspace)* Update for `anton-rs` org transfer ([#474](https://github.com/anton-rs/kona/pull/474))
- *(workspace)* Hoist Dependencies ([#466](https://github.com/anton-rs/kona/pull/466))
- *(bin)* Remove `kt` ([#461](https://github.com/anton-rs/kona/pull/461))
- refactor types out of kona-derive ([#454](https://github.com/anton-rs/kona/pull/454))
- bump scr version ([#440](https://github.com/anton-rs/kona/pull/440))
- Bump `superchain-registry` version ([#306](https://github.com/anton-rs/kona/pull/306))

## [0.0.1](https://github.com/anton-rs/kona/releases/tag/kona-primitives-v0.0.1) - 2024-06-22

### Added
- *(kona-derive)* Towards Derivation ([#243](https://github.com/anton-rs/kona/pull/243))
- *(ci)* Dependabot config ([#236](https://github.com/anton-rs/kona/pull/236))
- *(client)* `StatelessL2BlockExecutor` ([#210](https://github.com/anton-rs/kona/pull/210))
- *(primitives)* move attributes into primitives ([#163](https://github.com/anton-rs/kona/pull/163))
- *(plasma)* Implements Plasma Support for kona derive ([#152](https://github.com/anton-rs/kona/pull/152))
- *(primitives)* kona-derive type refactor ([#135](https://github.com/anton-rs/kona/pull/135))

### Fixed
- use 2718 encoding ([#231](https://github.com/anton-rs/kona/pull/231))
- Strong Error Typing ([#187](https://github.com/anton-rs/kona/pull/187))
- *(primitives)* use decode_2718() to gracefully handle the tx type ([#182](https://github.com/anton-rs/kona/pull/182))
- *(ci)* Release plz ([#145](https://github.com/anton-rs/kona/pull/145))
- *(workspace)* Release plz ([#138](https://github.com/anton-rs/kona/pull/138))

### Other
- version dependencies ([#296](https://github.com/anton-rs/kona/pull/296))
- re-export input types ([#279](https://github.com/anton-rs/kona/pull/279))
- *(deps)* fast forward op alloy dep ([#267](https://github.com/anton-rs/kona/pull/267))
- use alloy withdrawal type ([#213](https://github.com/anton-rs/kona/pull/213))
