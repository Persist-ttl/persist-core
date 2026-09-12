---
name: Bug report
about: Report a false positive/negative, crash, or incorrect output
title: "[bug] "
labels: bug
---

**What happened**

A clear description of the incorrect behavior (false positive, false
negative, crash, wrong severity/score, malformed output, etc).

**Minimal contract snippet**

```rust
// The smallest Soroban contract snippet that reproduces the issue.
```

**Command run**

```sh
cargo persist-audit ...
```

**Expected vs actual output**

**Environment**

- `cargo persist-audit --version`:
- OS:
