# Persist

**Static analysis for Soroban storage-lifecycle (TTL/archival) bugs.**

[![CI](https://github.com/Persist-ttl/persist-core/actions/workflows/ci.yml/badge.svg)](https://github.com/Persist-ttl/persist-core/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/cargo-persist-audit.svg)](https://crates.io/crates/cargo-persist-audit)

## The problem

Soroban's storage model is fundamentally different from EVM's. Every storage
entry — Persistent, Temporary, or Instance — has a **Time To Live (TTL)**,
measured in ledgers. If that TTL expires without being extended
(`extend_ttl`), the entry becomes **archived** and is effectively
unreadable, or in the case of Temporary storage, permanently deleted,
without manual restoration.

This causes real bugs: contracts silently losing state, functions
reverting unexpectedly months after deployment, and balances or allowances
becoming inaccessible. Existing tools (Scout, Komet, Certora) cover generic
issues — overflow, unsafe unwraps, reentrancy-adjacent patterns — but none
of them have dedicated detectors for storage-lifecycle correctness. Persist
exists to fill that gap.

## Install

```sh
# from crates.io, once published
cargo install cargo-persist-audit

# until then, install directly from this repo
cargo install --git https://github.com/Persist-ttl/persist-core cargo-persist-audit
```

## Usage

```sh
# scan the current directory (or pass a file/directory path)
cargo persist-audit

# choose an output format: text (default), json, md, sarif
cargo persist-audit ./contracts --format sarif --output report.sarif

# fail CI when Critical findings are present (the default);
# pass `none` to always exit 0, or high/medium/low to gate more strictly
cargo persist-audit --fail-on critical

# tune the "extend_ttl looks too small" threshold (in ledgers)
cargo persist-audit --min-ttl-threshold 17280
```

## Detectors (v1)

| Detector | Severity | Description |
|---|---|---|
| [`missing-ttl-extension`](crates/persist-analyzer/src/detectors/missing_ttl_extension.rs) | Critical (Persistent) / High (Instance) | A `.set()` on Persistent/Instance storage with no `extend_ttl` call anywhere in the same function or `impl` block. |
| [`insufficient-ttl-margin`](crates/persist-analyzer/src/detectors/insufficient_ttl_margin.rs) | Medium | An `extend_ttl` call whose hardcoded extension is below a safe ledger-count threshold. |
| [`temporary-storage-misuse`](crates/persist-analyzer/src/detectors/temporary_storage_misuse.rs) | Medium/High | Temporary storage is read for a key that's never (re)written, so the code assumes persistence Temporary storage doesn't provide. |
| [`no-archival-handling`](crates/persist-analyzer/src/detectors/no_archival_handling.rs) | High | A `.get()` result is immediately `.unwrap()`/`.expect()`-ed, with no handling for a missing/archived entry. |
| [`storage-type-mismatch`](crates/persist-analyzer/src/detectors/storage_type_mismatch.rs) | Low/Medium | A key name suggests contract-wide config stored outside Instance storage, or per-user data stored as Instance storage. |

Every detector has a paired real, compiling example under
[`test-cases/`](test-cases) — a minimal Soroban contract that triggers it
(`vulnerable/`) and the same contract fixed (`fixed/`) — used both as the
integration test suite and as executable documentation.

## Output formats

`--format text` (default, colorized terminal output), `json`, `md`
(shaped for a GitHub PR comment), and `sarif` (for GitHub's *Security >
Code Scanning* tab).

Every report includes a **security score (0-100)**: 100 minus a weighted
deduction per finding (Critical -25, High -15, Medium -8, Low -3, floored
at 0).

## How it works

`cargo-persist-audit` parses each `.rs` file with [`syn`](https://docs.rs/syn)
and pattern-matches method-call chains shaped like
`<env>.storage().<persistent|instance|temporary>().<op>(...)`, which is the
exact and only shape the `soroban-sdk` storage API takes. Each detector then
runs against that lightweight per-file model.

This is a deliberate design choice: Persist does **syntactic, AST-level
analysis**, not a fully type-checked rustc/dylint pipeline. That keeps the
tool fast, dependency-light, and easy to extend with a new detector in a
single file — at the cost of being heuristic in a few places (documented in
each detector's module doc comment), such as grouping "the same contract"
by `impl` block rather than doing real call-graph/interprocedural analysis.
A move to a type-resolved backend is on the roadmap below if the syntactic
approach turns out to be too noisy in practice.

## Roadmap

- TTL-cost estimation (estimate rent cost of a given extension in XLM).
- Auto-fix suggestions (`--fix` to insert a reasonable `extend_ttl` call).
- A 6th detector: TTL extension calls that are unreachable (dead code) on
  the read path that actually matters.
- Optional type-resolved backend for lower false-positive/negative rates.

## Project ecosystem

| Repo | What it is |
|---|---|
| [`persist-core`](https://github.com/Persist-ttl/persist-core) | This repo: the analysis engine and `cargo persist-audit` CLI. |
| [`persist-actions`](https://github.com/Persist-ttl/persist-actions) | GitHub Action that runs Persist in CI, comments on PRs, and uploads SARIF. |
| [`persist-vs`](https://github.com/Persist-ttl/persist-vs) | VS Code extension: inline diagnostics from Persist on save. |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to add a new detector.

## License

MIT, see [LICENSE](LICENSE).
