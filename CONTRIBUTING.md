# Contributing to Mutanchor

Thanks for considering contributing to Mutanchor. This project is small by
design — a deterministic mutation-testing CLI for Solana Anchor programs — so
the bar for a good contribution is: it makes the tool more correct, more
reliable, or easier to use, and it ships with evidence.

## Ground rules

1. **Deterministic, no AI in the analysis path.** Changes to the mutation
   engine must stay a fixed rule set. Same input → same mutants, every time.
   AI may build the tool; it must never decide whether a change was caught.
2. **One fault per mutant.** A contribution that produces two faults per
   mutant, or meaningless (behavior-unchanging) mutants, is rejected — it makes
   the score unreadable.
3. **No mocks, no sample data, no secrets.** Everything in the repo must be
   real. The report panel stays empty until a real run exists. Never commit
   keys or `.env`.
4. **README = spec; fix code to match it, not docs to match the code.**
5. **Conventional commits.** `feat:`, `fix:`, `test:`, `docs:`, `chore:`,
   `ci:`, `refactor:`. One logical change per commit.

## Getting started

```bash
git clone https://github.com/subheeksh5599/mutanchor.git
cd mutanchor
cargo build
cargo test
```

Prerequisites: Rust stable (2021 edition). For full mutation **execution**
(not just the engine), you need the Solana toolchain (`cargo build-sbf`) and a
machine or CI runner big enough to build Anchor programs repeatedly — the
2-core laptop this was built on times out on per-mutant SBF builds, which is
why the CI demo-job exists.

## What to work on

- A bug or gap in an existing operator (add a failing unit test first).
- A new operator that models a real Solana audit-findings bug class (cite it).
- Runner reliability: build-cache correctness, timeout handling, kill/restore
  cleanliness, honest verdicts.
- Test coverage: kill-path integration tests, golden-file report tests,
  property tests (every mutant resolves, none hang).
- Docs and repo polish (this list stays honest).

## Before you open a PR

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes (all suites)
- [ ] You added a test for your change (a fix without a test is a regression)
- [ ] Your README/claims match what the code actually does

## Review process

PRs are reviewed line by line. A review tick means the thing is genuinely
fixed **and** tested — not just claimed. Expect 1–2 revision cycles; that's
normal, not rejection.

## Code of conduct

See CODE_OF_CONDUCT.md.
