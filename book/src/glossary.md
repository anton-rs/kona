# Glossary

*This document contains definitions for terms used throughout the Kona book.*

#### Fault Proof VM
A `Fault Proof VM` is a virtual machine, commonly supporting a modified subset of an existing reduced instruction set architecture,
that is designed to execute verifiable programs.

Full specification for the `cannon` & `cannon-rs` FPVMs, as an example, is available in the [Optimism Monorepo][cannon-specs].

#### Fault Proof Program
A `Fault Proof Program` is a program, commonly written in a general purpose language such as Golang, C, or Rust, that may be compiled down
to a compatible `Fault Proof VM` target and provably executed on that target VM.

Examples of `Fault Proof Programs` include the [OP Program][op-program], which runs on top of [`cannon`][cannon], [`cannon-rs`][cannon-rs], and
[`asterisc`][asterisc] to verify a claim about the state of an [OP Stack][op-stack] layer two.

{{#include ../static/links.md}}
