# Lint the workspace
lint:
  cargo +nightly fmt --all && cargo +nightly clippy --all --all-features --all-targets -- -D warnings
