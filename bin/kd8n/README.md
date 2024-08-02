# `kd8n`

`kd8n`, or _Kona Derivation_, is a binary runner for [`kona-derive`][kd] against
the [`op-test-vectors`][opt] test suite.

It can be run over specific derivation test fixtures or the entire [`op-test-vectors`][opt]
test suite.

The design of `kd8n` is inspired by [`revme`][revme].

[kd]: ../../crates/derive
[opt]: https://github.com/ethereum-optimism/op-test-vectors
[revme]: https://github.com/bluealloy/revm/tree/main/bins/revme

## Usage

Run all [`op-test-vectors`][opt] derivation test fixtures against `kona-derive`.

```bash
$ kd8n --all
```
