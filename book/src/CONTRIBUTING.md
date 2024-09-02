# Contributing

Thank you for wanting to contribute! Before contributing to this repository, please read through this document and
discuss the change you wish to make via issue or in the development telegram.

## Dependencies

Before working with this repository locally, you'll need to install several dependencies:

- [Docker](https://www.docker.com/) for cross-compilation.
- [just](https://github.com/casey/just) for our command-runner scripts.
- The [Rust toolchain](https://rustup.rs/)
- The [Golang toolchain](https://go.dev/dl/)

**Optional**

- [mdbook](https://github.com/rust-lang/mdBook) to contribute to the [book](/)
  - [mdbook-template](https://github.com/sgoudham/mdbook-template)
  - [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid)

## Pull Request Process

1. Before anything, [create an issue](https://github.com/anton-rs/kona/issues/new) to discuss the change you're
   wanting to make, if it is significant or changes functionality. Feel free to skip this step for trivial changes.
1. Once your change is implemented, ensure that all checks are passing before creating a PR. The full CI pipeline can
   be ran locally via the `justfile`s in the repository.
1. Make sure to update any documentation that has gone stale as a result of the change, in the `README` files, the [book][book],
   and in rustdoc comments.
1. Once you have sign-off from a maintainer, you may merge your pull request yourself if you have permissions to do so.
   If not, the maintainer who approves your pull request will add it to the merge queue.
