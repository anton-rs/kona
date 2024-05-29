# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2](https://github.com/ethereum-optimism/kona/compare/kona-common-v0.0.1...kona-common-v0.0.2) - 2024-05-29

### Added
- *(host)* Host program scaffold ([#184](https://github.com/ethereum-optimism/kona/pull/184))
- *(preimage)* `OracleServer` + `HintReader` ([#96](https://github.com/ethereum-optimism/kona/pull/96))
- *(common)* Move from `RegisterSize` to native ptr size type ([#95](https://github.com/ethereum-optimism/kona/pull/95))
- *(workspace)* Add `rustfmt.toml`

### Other
- *(common)* Use `Box::leak` rather than `mem::forget` ([#180](https://github.com/ethereum-optimism/kona/pull/180))
- Add simple blocking async executor ([#38](https://github.com/ethereum-optimism/kona/pull/38))
- Make versions of packages independent ([#36](https://github.com/ethereum-optimism/kona/pull/36))
