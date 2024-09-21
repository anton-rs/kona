# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.4](https://github.com/anton-rs/kona/compare/kona-mpt-v0.0.3...kona-mpt-v0.0.4) - 2024-09-21

### Added

- *(mpt)* Migrate to `thiserror` ([#541](https://github.com/anton-rs/kona/pull/541))

### Fixed

- *(mpt)* Empty list walker ([#493](https://github.com/anton-rs/kona/pull/493))

## [0.0.3](https://github.com/anton-rs/kona/compare/kona-mpt-v0.0.2...kona-mpt-v0.0.3) - 2024-09-04

### Added
- *(mpt)* `TrieNode` benchmarks ([#351](https://github.com/anton-rs/kona/pull/351))

### Fixed
- *(workspace)* Add Unused Dependency Lint ([#453](https://github.com/anton-rs/kona/pull/453))
- *(deps)* Bump Alloy Dependencies ([#409](https://github.com/anton-rs/kona/pull/409))

### Other
- *(workspace)* Alloy Version Bumps ([#467](https://github.com/anton-rs/kona/pull/467))
- *(workspace)* Update for `anton-rs` org transfer ([#474](https://github.com/anton-rs/kona/pull/474))
- *(workspace)* Hoist Dependencies ([#466](https://github.com/anton-rs/kona/pull/466))
- *(bin)* Remove `kt` ([#461](https://github.com/anton-rs/kona/pull/461))
- *(deps)* Bump revm version to v13 ([#422](https://github.com/anton-rs/kona/pull/422))

## [0.0.2](https://github.com/anton-rs/kona/compare/kona-mpt-v0.0.1...kona-mpt-v0.0.2) - 2024-06-22

### Added
- *(client)* Derivation integration ([#257](https://github.com/anton-rs/kona/pull/257))
- *(client)* Oracle-backed derive traits ([#252](https://github.com/anton-rs/kona/pull/252))
- *(client)* Account + Account storage hinting in `TrieDB` ([#228](https://github.com/anton-rs/kona/pull/228))
- *(client)* Add `current_output_root` to block executor ([#225](https://github.com/anton-rs/kona/pull/225))
- *(ci)* Dependabot config ([#236](https://github.com/anton-rs/kona/pull/236))
- *(client)* `StatelessL2BlockExecutor` ([#210](https://github.com/anton-rs/kona/pull/210))
- *(mpt)* Block hash walkback ([#199](https://github.com/anton-rs/kona/pull/199))
- *(mpt)* Simplify `TrieDB` ([#198](https://github.com/anton-rs/kona/pull/198))
- *(mpt)* Trie DB commit ([#196](https://github.com/anton-rs/kona/pull/196))
- *(mpt)* Trie node insertion ([#195](https://github.com/anton-rs/kona/pull/195))
- *(host)* Host program scaffold ([#184](https://github.com/anton-rs/kona/pull/184))
- *(workspace)* Client programs in workspace ([#178](https://github.com/anton-rs/kona/pull/178))
- *(mpt)* `TrieCacheDB` scaffold ([#174](https://github.com/anton-rs/kona/pull/174))
- *(mpt)* `TrieNode` retrieval ([#173](https://github.com/anton-rs/kona/pull/173))
- *(mpt)* Refactor `TrieNode` ([#172](https://github.com/anton-rs/kona/pull/172))

### Fixed
- *(mpt)* Fix extension node truncation ([#300](https://github.com/anton-rs/kona/pull/300))
- *(ci)* Release plz ([#145](https://github.com/anton-rs/kona/pull/145))

### Other
- version dependencies ([#296](https://github.com/anton-rs/kona/pull/296))
- *(mpt)* Do not expose recursion vars ([#197](https://github.com/anton-rs/kona/pull/197))
