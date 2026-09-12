# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added

- Initial `cargo-persist-audit` CLI (`cargo persist-audit`).
- `syn`-based storage-op AST model recognizing `soroban_sdk::storage` API
  call chains.
- Five v1 detectors: `missing-ttl-extension`, `insufficient-ttl-margin`,
  `temporary-storage-misuse`, `no-archival-handling`, `storage-type-mismatch`.
- Text, JSON, Markdown, and SARIF output formats, plus a 0-100 security
  score.
- Paired real, compiling vulnerable/fixed Soroban contract examples for
  every detector under `test-cases/`.
