# Changelog

All notable changes to Mutanchor are documented here.

## [0.1.0] — 2026-08-19

### Added
- **Mutation engine** — deterministic source rewriting with the full 8-operator
  rule set, one fault per mutant, with `file:line` provenance and
  equivalent-mutant de-duplication ("meaningless mutants are dropped").
  - signer check removal
  - PDA seed swap
  - bump mismatch
  - authority check drop
  - discriminator removal
  - CPI target swap
  - close/rent check drop
  - comparison / boundary flip (with generic-lifetime guarding)
- **`init`** — maps every Anchor instruction to its source file and line range,
  including the account-constraint structs used by each handler.
- **`run`** — builds each mutant with `cargo build-sbf` (Solana's official
  toolchain), executes the program's own test suite against it on LiteSVM via
  `MUTANCHOR_PROGRAM_SO`, and classifies every mutant as
  killed / survived / build-failed / timeout.
- **`report`** — per-instruction and overall mutation score, surviving-mutant
  annotations with attacker exploit sketches, rendered as HTML and JSON, plus a
  `dashboard.json` for the live report panel.
- **`ci`** — fails the build when surviving mutants exceed a threshold.
- Unit tests for every operator on small fixture programs.
- GitHub Actions CI: build, test, rustfmt, clippy, cargo audit, and a demo-job
  that runs the mutator on the demo program and publishes the report artifact.

### Changed
- The CLI's `init/run/report/ci` subcommands grew from a scaffold that printed
  "not implemented" into a working mutation pipeline (see README honesty table:
  previously-pending phases 1, 2, 3 are now implemented).

### Fixed
- Comparison-flip no longer corrupts generic/lifetime syntax (e.g.
  `Account<'_, Vault>` is never treated as a `<` comparison).
