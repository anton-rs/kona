# `kona-plasma`

Plasma Data Availability Adapter for `kona-derive`.

[plasma]: https://specs.optimism.io/experimental/plasma.html

`kona-plasma` is an implementation of the [Plasma][plasma] OP Stack Specification in rust.

## Usage

Add `kona-plasma` to your `Cargo.toml`.

```ignore
[dependencies]
kona-plasma = "0.0.1"

# Serde is enabled by default and can be disabled by toggling default-features off
# kona-plasma = { version = "0.0.1", default-features = false }
```

## Features

### Serde

[`serde`] serialization and deserialization support for `kona-plasma` types.

By default, the `serde` feature is enabled on `kona-plasma`.
