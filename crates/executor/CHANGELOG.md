# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3](https://github.com/anton-rs/kona/compare/kona-executor-v0.0.2...kona-executor-v0.0.3) - 2024-10-03

### Added

- *(executor)* Use EIP-1559 parameters from payload attributes ([#616](https://github.com/anton-rs/kona/pull/616))
- *(derive)* bump op-alloy dep ([#605](https://github.com/anton-rs/kona/pull/605))
- kona-providers ([#596](https://github.com/anton-rs/kona/pull/596))
- *(executor)* Migrate to `thiserror` ([#544](https://github.com/anton-rs/kona/pull/544))
- *(mpt)* Migrate to `thiserror` ([#541](https://github.com/anton-rs/kona/pull/541))
- *(primitives)* Remove Attributes ([#529](https://github.com/anton-rs/kona/pull/529))
- large dependency update ([#528](https://github.com/anton-rs/kona/pull/528))

### Fixed

- *(workspace)* hoist and fix lints ([#577](https://github.com/anton-rs/kona/pull/577))

### Other

- doc logos ([#609](https://github.com/anton-rs/kona/pull/609))
- *(workspace)* Allow stdlib in `cfg(test)` ([#548](https://github.com/anton-rs/kona/pull/548))
- Bumps Dependency Versions ([#520](https://github.com/anton-rs/kona/pull/520))
- *(primitives)* rm RawTransaction ([#505](https://github.com/anton-rs/kona/pull/505))

## [0.0.2](https://github.com/anton-rs/kona/compare/kona-executor-v0.0.1...kona-executor-v0.0.2) - 2024-09-04

### Added
- *(executor)* Expose full revm Handler ([#475](https://github.com/anton-rs/kona/pull/475))
- *(workspace)* Workspace Re-exports ([#468](https://github.com/anton-rs/kona/pull/468))
- *(executor)* `StatelessL2BlockExecutor` benchmarks ([#350](https://github.com/anton-rs/kona/pull/350))
- *(executor)* Generic precompile overrides ([#340](https://github.com/anton-rs/kona/pull/340))
- *(executor)* Builder pattern for `StatelessL2BlockExecutor` ([#339](https://github.com/anton-rs/kona/pull/339))

### Fixed
- *(workspace)* Use published `revm` version ([#459](https://github.com/anton-rs/kona/pull/459))
- downgrade for release plz ([#458](https://github.com/anton-rs/kona/pull/458))
- *(workspace)* Add Unused Dependency Lint ([#453](https://github.com/anton-rs/kona/pull/453))
- Don't hold onto intermediate execution cache across block boundaries ([#396](https://github.com/anton-rs/kona/pull/396))

### Other
- *(workspace)* Alloy Version Bumps ([#467](https://github.com/anton-rs/kona/pull/467))
- *(workspace)* Update for `anton-rs` org transfer ([#474](https://github.com/anton-rs/kona/pull/474))
- *(workspace)* Hoist Dependencies ([#466](https://github.com/anton-rs/kona/pull/466))
- refactor types out of kona-derive ([#454](https://github.com/anton-rs/kona/pull/454))
- *(deps)* Bump revm version to v13 ([#422](https://github.com/anton-rs/kona/pull/422))

## [0.0.1](https://github.com/anton-rs/kona/releases/tag/kona-executor-v0.0.1) - 2024-06-22

### Other
- *(workspace)* Prep release ([#301](https://github.com/anton-rs/kona/pull/301))
- version dependencies ([#296](https://github.com/anton-rs/kona/pull/296))
- *(deps)* fast forward op alloy dep ([#267](https://github.com/anton-rs/kona/pull/267))
- *(workspace)* `kona-executor` ([#259](https://github.com/anton-rs/kona/pull/259))
