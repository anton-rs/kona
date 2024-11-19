# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.5](https://github.com/anton-rs/kona/compare/kona-common-v0.0.4...kona-common-v0.0.5) - 2024-11-19

### Other

- *(workspace)* Reorganize SDK ([#816](https://github.com/anton-rs/kona/pull/816))

## [0.0.4](https://github.com/anton-rs/kona/compare/kona-common-v0.0.3...kona-common-v0.0.4) - 2024-10-25

### Added

- remove thiserror ([#735](https://github.com/anton-rs/kona/pull/735))
- *(preimage/common)* Migrate to `thiserror` ([#543](https://github.com/anton-rs/kona/pull/543))

### Fixed

- *(workspace)* hoist and fix lints ([#577](https://github.com/anton-rs/kona/pull/577))

### Other

- re-org imports ([#711](https://github.com/anton-rs/kona/pull/711))
- *(preimage)* Test Coverage ([#634](https://github.com/anton-rs/kona/pull/634))
- test coverage for common ([#629](https://github.com/anton-rs/kona/pull/629))
- doc logos ([#609](https://github.com/anton-rs/kona/pull/609))
- *(workspace)* Allow stdlib in `cfg(test)` ([#548](https://github.com/anton-rs/kona/pull/548))

## [0.0.3](https://github.com/anton-rs/kona/compare/kona-common-v0.0.2...kona-common-v0.0.3) - 2024-09-04

### Added
- add zkvm target for io ([#394](https://github.com/anton-rs/kona/pull/394))

### Other
- *(workspace)* Update for `anton-rs` org transfer ([#474](https://github.com/anton-rs/kona/pull/474))
- *(workspace)* Hoist Dependencies ([#466](https://github.com/anton-rs/kona/pull/466))
- *(bin)* Remove `kt` ([#461](https://github.com/anton-rs/kona/pull/461))
- *(common)* Remove need for cursors in `NativeIO` ([#416](https://github.com/anton-rs/kona/pull/416))

## [0.0.2](https://github.com/anton-rs/kona/compare/kona-common-v0.0.1...kona-common-v0.0.2) - 2024-06-22

### Added
- *(client)* Derivation integration ([#257](https://github.com/anton-rs/kona/pull/257))
- *(client/host)* Oracle-backed Blob fetcher ([#255](https://github.com/anton-rs/kona/pull/255))
- *(host)* Host program scaffold ([#184](https://github.com/anton-rs/kona/pull/184))
- *(preimage)* `OracleServer` + `HintReader` ([#96](https://github.com/anton-rs/kona/pull/96))
- *(common)* Move from `RegisterSize` to native ptr size type ([#95](https://github.com/anton-rs/kona/pull/95))
- *(workspace)* Add `rustfmt.toml`

### Fixed
- *(common)* Pipe IO support ([#282](https://github.com/anton-rs/kona/pull/282))

### Other
- *(common)* Use `Box::leak` rather than `mem::forget` ([#180](https://github.com/anton-rs/kona/pull/180))
- Add simple blocking async executor ([#38](https://github.com/anton-rs/kona/pull/38))
- Make versions of packages independent ([#36](https://github.com/anton-rs/kona/pull/36))
