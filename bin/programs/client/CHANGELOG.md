# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/ethereum-optimism/kona/releases/tag/kona-client-v0.1.0) - 2024-06-16

### Added
- *(client)* Derivation integration ([#257](https://github.com/ethereum-optimism/kona/pull/257))
- *(client/host)* Oracle-backed Blob fetcher ([#255](https://github.com/ethereum-optimism/kona/pull/255))
- *(client)* Oracle-backed derive traits ([#252](https://github.com/ethereum-optimism/kona/pull/252))
- *(client)* Add `RollupConfig` to `BootInfo` ([#251](https://github.com/ethereum-optimism/kona/pull/251))
- *(kona-derive)* Towards Derivation ([#243](https://github.com/ethereum-optimism/kona/pull/243))
- *(client)* Account + Account storage hinting in `TrieDB` ([#228](https://github.com/ethereum-optimism/kona/pull/228))
- *(client)* Add `current_output_root` to block executor ([#225](https://github.com/ethereum-optimism/kona/pull/225))
- *(client)* `StatelessL2BlockExecutor` ([#210](https://github.com/ethereum-optimism/kona/pull/210))
- *(client)* `BootInfo` ([#205](https://github.com/ethereum-optimism/kona/pull/205))
- *(host)* Host program scaffold ([#184](https://github.com/ethereum-optimism/kona/pull/184))

### Fixed
- output root version to 32 bytes ([#248](https://github.com/ethereum-optimism/kona/pull/248))

### Other
- *(workspace)* `kona-executor` ([#259](https://github.com/ethereum-optimism/kona/pull/259))
- *(host)* Simplify host program ([#206](https://github.com/ethereum-optimism/kona/pull/206))
