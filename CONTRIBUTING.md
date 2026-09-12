# Contributing to persist-core

## Adding a new detector

1. Create `crates/persist-analyzer/src/detectors/<slug>.rs` (use
   `snake_case` for the file, `kebab-case` for the detector's `slug`).
2. Export a `pub const INFO: DetectorInfo` describing the detector, and a
   `pub fn detect(file_path: &str, model: &FileModel, ...) -> Vec<Finding>`
   (add a `&Config` argument only if the detector needs a configurable
   threshold, following `insufficient_ttl_margin.rs`).
3. Register the module in `crates/persist-analyzer/src/detectors/mod.rs`:
   add it to `ALL_DETECTORS` and call it from `run_all`.
4. Write inline unit tests in a `#[cfg(test)] mod tests` block at the
   bottom of your detector file, each using `syn::parse_str` on a small
   inline snippet — cover at least one case that should fire and one that
   shouldn't.
5. Add a real, compiling example pair under `test-cases/<slug>/{vulnerable,fixed}/`:
   - Copy the `Cargo.toml` shape from an existing example (a `soroban-sdk`
     dependency plus a `soroban-sdk` dev-dependency with the `testutils`
     feature).
   - Write a minimal `#![no_std]` contract that triggers your detector in
     `vulnerable/src/lib.rs`, and the same contract fixed in
     `fixed/src/lib.rs`. Each should have its own passing `#[test]`.
   - Add both crates to `test-cases/Cargo.toml`'s `members` list.
   - Add a `fixture_test!(...)` entry in
     `crates/persist-analyzer/tests/detectors_against_fixtures.rs` pointing
     at your new files.
6. Add a row to the detector table in `README.md`.
7. Run `cargo test` (from `persist-core/`) and `cargo test` (from
   `persist-core/test-cases/`) and make sure both are green.

## Code style

Run `cargo fmt` and `cargo clippy --all-targets` before opening a PR. CI
runs both.

## Reporting bugs

Please include the contract snippet that produced the false
positive/negative, the command you ran, and the `persist-core` version
(`cargo persist-audit --version`).
