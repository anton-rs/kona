# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3](https://github.com/ethereum-optimism/kona/compare/kona-derive-v0.0.2...kona-derive-v0.0.3) - 2024-06-27

### Added
- *(derive)* Stage Level Metrics ([#309](https://github.com/ethereum-optimism/kona/pull/309))
- *(examples)* Trusted Sync Metrics ([#308](https://github.com/ethereum-optimism/kona/pull/308))

### Fixed
- *(derive)* Warnings with metrics macro ([#322](https://github.com/ethereum-optimism/kona/pull/322))

### Other
- clean up trusted sync loop ([#318](https://github.com/ethereum-optimism/kona/pull/318))
- *(docs)* Label Cleanup ([#307](https://github.com/ethereum-optimism/kona/pull/307))
- *(derive)* add targets to stage logs ([#310](https://github.com/ethereum-optimism/kona/pull/310))

## [0.0.2](https://github.com/ethereum-optimism/kona/compare/kona-derive-v0.0.1...kona-derive-v0.0.2) - 2024-06-22

### Added
- *(fjord)* fjord parameter changes ([#284](https://github.com/ethereum-optimism/kona/pull/284))
- *(client/host)* Oracle-backed Blob fetcher ([#255](https://github.com/ethereum-optimism/kona/pull/255))
- *(kona-derive)* Towards Derivation ([#243](https://github.com/ethereum-optimism/kona/pull/243))
- *(kona-derive)* Updated interface ([#230](https://github.com/ethereum-optimism/kona/pull/230))
- *(ci)* Dependabot config ([#236](https://github.com/ethereum-optimism/kona/pull/236))
- *(client)* `StatelessL2BlockExecutor` ([#210](https://github.com/ethereum-optimism/kona/pull/210))
- Pipeline Builder ([#217](https://github.com/ethereum-optimism/kona/pull/217))
- Minimal ResetProvider Implementation ([#208](https://github.com/ethereum-optimism/kona/pull/208))
- refactor the pipeline builder ([#209](https://github.com/ethereum-optimism/kona/pull/209))
- refactor reset provider ([#207](https://github.com/ethereum-optimism/kona/pull/207))
- *(preimage)* Async server components ([#183](https://github.com/ethereum-optimism/kona/pull/183))
- *(workspace)* Client programs in workspace ([#178](https://github.com/ethereum-optimism/kona/pull/178))
- *(primitives)* move attributes into primitives ([#163](https://github.com/ethereum-optimism/kona/pull/163))
- *(derive)* return the concrete online attributes queue type from the online stack constructor ([#158](https://github.com/ethereum-optimism/kona/pull/158))
- *(derive)* Abstract Alt DA out of `kona-derive` ([#156](https://github.com/ethereum-optimism/kona/pull/156))
- *(derive)* Online Data Source Factory Wiring ([#150](https://github.com/ethereum-optimism/kona/pull/150))
- *(plasma)* Implements Plasma Support for kona derive ([#152](https://github.com/ethereum-optimism/kona/pull/152))
- *(derive)* Pipeline Builder ([#127](https://github.com/ethereum-optimism/kona/pull/127))
- *(primitives)* kona-derive type refactor ([#135](https://github.com/ethereum-optimism/kona/pull/135))
- *(derive)* Span Batch Validation ([#121](https://github.com/ethereum-optimism/kona/pull/121))
- *(derive)* Use `L2ChainProvider` for system config fetching in attributes builder ([#123](https://github.com/ethereum-optimism/kona/pull/123))
- *(derive)* Online Blob Provider ([#117](https://github.com/ethereum-optimism/kona/pull/117))
- *(derive)* payload builder tests ([#106](https://github.com/ethereum-optimism/kona/pull/106))
- *(derive)* deposit derivation testing ([#115](https://github.com/ethereum-optimism/kona/pull/115))
- *(derive)* Build `L1BlockInfoTx` in payload builder ([#102](https://github.com/ethereum-optimism/kona/pull/102))
- *(derive)* `L2ChainProvider` w/ `op-alloy-consensus` ([#98](https://github.com/ethereum-optimism/kona/pull/98))
- *(derive)* Add `L1BlockInfoTx` ([#100](https://github.com/ethereum-optimism/kona/pull/100))
- *(derive)* Payload Attribute Building ([#92](https://github.com/ethereum-optimism/kona/pull/92))
- *(derive)* Online `ChainProvider` ([#93](https://github.com/ethereum-optimism/kona/pull/93))
- *(derive)* Move to `tracing` for telemetry ([#94](https://github.com/ethereum-optimism/kona/pull/94))
- *(derive)* Batch Queue Logging ([#86](https://github.com/ethereum-optimism/kona/pull/86))
- *(derive)* Add `ecrecover` trait + features ([#90](https://github.com/ethereum-optimism/kona/pull/90))
- *(derive)* Use upstream alloy ([#89](https://github.com/ethereum-optimism/kona/pull/89))
- *(derive)* add next_attributes test
- *(workspace)* Add `rustfmt.toml`
- *(derive)* `SpanBatch` type implementation WIP
- *(derive)* Reorganize modules
- *(derive)* `add_txs` function
- *(derive)* Derive raw batches, mocks
- *(derive)* Refactor serialization; `SpanBatchPayload` WIP
- *(derive)* fixed bytes and encoding
- *(derive)* raw span type refactoring
- *(types)* span batches
- *(derive)* Channel Reader Implementation ([#65](https://github.com/ethereum-optimism/kona/pull/65))
- *(derive)* share the rollup config across stages using an arc
- *(derive)* Test Utilities ([#62](https://github.com/ethereum-optimism/kona/pull/62))
- Single batch type ([#43](https://github.com/ethereum-optimism/kona/pull/43))
- *(derive)* channel bank ([#46](https://github.com/ethereum-optimism/kona/pull/46))
- Frame queue stage ([#45](https://github.com/ethereum-optimism/kona/pull/45))
- L1 retrieval ([#44](https://github.com/ethereum-optimism/kona/pull/44))
- System config update event parsing ([#42](https://github.com/ethereum-optimism/kona/pull/42))
- Add OP receipt fields ([#41](https://github.com/ethereum-optimism/kona/pull/41))
- Add `TxDeposit` type ([#40](https://github.com/ethereum-optimism/kona/pull/40))
- L1 traversal ([#39](https://github.com/ethereum-optimism/kona/pull/39))

### Fixed
- *(derive)* Fjord brotli decompression ([#298](https://github.com/ethereum-optimism/kona/pull/298))
- *(examples)* Dynamic Rollup Config Loading ([#293](https://github.com/ethereum-optimism/kona/pull/293))
- type re-exports ([#280](https://github.com/ethereum-optimism/kona/pull/280))
- *(kona-derive)* reuse upstream reqwest provider ([#229](https://github.com/ethereum-optimism/kona/pull/229))
- Derivation Pipeline ([#220](https://github.com/ethereum-optimism/kona/pull/220))
- *(derive)* Alloy EIP4844 Blob Type ([#215](https://github.com/ethereum-optimism/kona/pull/215))
- Strong Error Typing ([#187](https://github.com/ethereum-optimism/kona/pull/187))
- *(derive)* inline blob verification into the blob provider ([#175](https://github.com/ethereum-optimism/kona/pull/175))
- *(derive)* fix span batch utils read_tx_data() ([#170](https://github.com/ethereum-optimism/kona/pull/170))
- *(derive)* Ethereum Data Source ([#159](https://github.com/ethereum-optimism/kona/pull/159))
- *(derive)* remove unnecessary online feature decorator ([#160](https://github.com/ethereum-optimism/kona/pull/160))
- *(ci)* Release plz ([#145](https://github.com/ethereum-optimism/kona/pull/145))
- *(derive)* move span batch conversion to try from trait ([#142](https://github.com/ethereum-optimism/kona/pull/142))
- *(derive)* Small Fixes and Span Batch Validation Fix ([#139](https://github.com/ethereum-optimism/kona/pull/139))
- *(workspace)* Release plz ([#138](https://github.com/ethereum-optimism/kona/pull/138))
- *(workspace)* Release plz ([#137](https://github.com/ethereum-optimism/kona/pull/137))
- *(derive)* Rebase span batch validation tests ([#125](https://github.com/ethereum-optimism/kona/pull/125))
- *(derive)* Span batch bitlist encoding ([#122](https://github.com/ethereum-optimism/kona/pull/122))
- *(derive)* Doc Touchups and Telemetry ([#105](https://github.com/ethereum-optimism/kona/pull/105))
- *(derive)* Derive full `SpanBatch` in channel reader ([#97](https://github.com/ethereum-optimism/kona/pull/97))
- *(derive)* Stage Decoupling ([#88](https://github.com/ethereum-optimism/kona/pull/88))
- *(derive)* add back removed test
- *(derive)* lints
- *(derive)* extend attributes queue unit test
- *(derive)* successful payload attributes building tests
- *(derive)* error equality fixes and tests
- *(derive)* rework abstractions and attributes queue testing
- *(derive)* attributes queue
- *(derive)* hoist params
- *(derive)* merge upstream changes
- *(derive)* fix bricked arc stage param construction ([#84](https://github.com/ethereum-optimism/kona/pull/84))
- *(derive)* l1 retrieval docs ([#80](https://github.com/ethereum-optimism/kona/pull/80))
- *(derive)* clean up frame queue docs
- *(derive)* frame queue error bubbling and docs
- *(derive)* rebase
- *(derive)* merge upstream changes
- *(derive)* refactor tx enveloped
- *(derive)* refactor span batch tx types
- *(derive)* bitlist alignment
- *(derive)* span batch tx rlp
- *(derive)* span type encodings and decodings
- *(derive)* more types
- *(derive)* small l1 retrieval doc comment fix ([#61](https://github.com/ethereum-optimism/kona/pull/61))

### Other
- version dependencies ([#296](https://github.com/ethereum-optimism/kona/pull/296))
- payload decoding tests ([#287](https://github.com/ethereum-optimism/kona/pull/287))
- payload decoding tests ([#289](https://github.com/ethereum-optimism/kona/pull/289))
- re-export input types ([#279](https://github.com/ethereum-optimism/kona/pull/279))
- *(deps)* fast forward op alloy dep ([#267](https://github.com/ethereum-optimism/kona/pull/267))
- *(derive)* cleanup pipeline tracing ([#264](https://github.com/ethereum-optimism/kona/pull/264))
- *(derive)* online module touchups ([#265](https://github.com/ethereum-optimism/kona/pull/265))
- *(derive)* Sources Touchups ([#266](https://github.com/ethereum-optimism/kona/pull/266))
- *(kona-derive)* Online Pipeline Cleanup ([#241](https://github.com/ethereum-optimism/kona/pull/241))
- *(derive)* data source unit tests ([#181](https://github.com/ethereum-optimism/kona/pull/181))
- *(workspace)* Move `alloy-primitives` to workspace dependencies ([#103](https://github.com/ethereum-optimism/kona/pull/103))
- *(ci)* Fail CI on doclint failure ([#101](https://github.com/ethereum-optimism/kona/pull/101))
- *(derive)* cleanups ([#91](https://github.com/ethereum-optimism/kona/pull/91))
- Merge branch 'main' into refcell/data-sources
- Merge pull request [#87](https://github.com/ethereum-optimism/kona/pull/87) from ethereum-optimism/refcell/origin-providers
- Merge branch 'main' into refcell/channel-bank-tests
- Merge branch 'main' into refcell/payload-queue
- *(derive)* L1Traversal Doc and Test Cleanup ([#79](https://github.com/ethereum-optimism/kona/pull/79))
- Merge pull request [#67](https://github.com/ethereum-optimism/kona/pull/67) from ethereum-optimism/refcell/batch-queue
- *(derive)* Channel reader tests + fixes, batch type fixes
- *(derive)* `RawSpanBatch` diff decoding/encoding test
- *(derive)* rebase + move `alloy` module
- *(derive)* Clean up RLP encoding + use `TxType` rather than ints
- Update `derive` lint rules ([#47](https://github.com/ethereum-optimism/kona/pull/47))
- scaffold ([#37](https://github.com/ethereum-optimism/kona/pull/37))
- Make versions of packages independent ([#36](https://github.com/ethereum-optimism/kona/pull/36))
