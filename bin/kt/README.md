# `kt`

`kt` is a test-runner for the [`ethereum-optimism/tests`][opt] consensus tests.

The design of `kt` is inspired by [`revme`][revme].

## Usage

### `dn`

`dn`, or _Derivation_, is a runner for [`kona-derive`][kd] against the [`ethereum-optimism/tests`][opt] 
derivation test suite.

**Run all [`ethereum-optimism/tests`][opt] derivation test fixtures against `kona-derive`.**

```bash
$ kt dn --all
```

**Run specific derivation test fixture against `kona-derive`.**

```bash
$ kt dn --test <test_name>
```

### `t8n`

`t8n` is a runner for [`kona-executor`][ke] against the [`ethereum-optimism/tests`][opt] execution test suite.

**Run all [`ethereum-optimism/tests`][opt] execution test fixtures against `kona-executor`.**

```bash
$ kt t8n --all
```

**Run specific execution test fixture against `kona-executor`.**

```bash
$ kt t8n --test <test_name>
```

[ke]: ../../crates/executor
[kd]: ../../crates/derive
[opt]: https://github.com/ethereum-optimism/tests
[revme]: https://github.com/bluealloy/revm/tree/main/bins/revme
