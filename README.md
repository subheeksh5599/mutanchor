<div align="center">

# Mutanchor

[![License: MIT](https://img.shields.io/badge/license-MIT-14151a.svg)](LICENSE)
![Language](https://img.shields.io/badge/Rust%202021-14151a)
![Target](https://img.shields.io/badge/Solana%20Anchor-14151a)
![Status](https://img.shields.io/badge/status-early%20scaffold-9ca3af)

### Your tests pass. The bug ships anyway. Mutanchor proves which bugs your tests would miss.

Mutation testing for Solana Anchor programs. Mutanchor rewrites Anchor source at
the AST level to inject real bug classes — missing signer checks, PDA bump
mismatches, dropped authority checks, discriminator removal, boundary flips,
CPI target swaps — then builds each mutant with `cargo build-sbf` and runs the
program's test suite against it on LiteSVM. A mutant that survives means the
tests would miss that bug in production.

cargo-mutants has zero Solana support. No Solana mutation tool ships. Mutanchor
is that gap.

</div>

---

## Table of contents

- [See it in one command](#-see-it-in-one-command)
- [The problem Mutanchor solves](#the-problem-mutanchor-solves)
- [How Mutanchor works](#how-mutanchor-works)
- [Architecture](#architecture)
- [Operators](#operators)
- [What's real vs pending — the honesty table](#whats-real-vs-pending--the-honesty-table)
- [Tests](#tests)
- [Run it locally](#run-it-locally)
- [Project layout](#project-layout)
- [Tech stack](#tech-stack)
- [Roadmap](#roadmap)
- [License](#license)

---

## ▶ See it in one command

```bash
$ cargo run -- --help
Mutation testing for Solana Anchor programs

Usage: mutanchor <COMMAND>

Commands:
  init    Parse the Anchor IDL and locate instruction source files
  run     Mutate, build each mutant, and run the suite on LiteSVM
  report  Emit the mutation report (HTML + JSON)
  ci      CI mode: fail when surviving mutants exceed a threshold
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

The CLI is the contract. `init` locates what to mutate, `run` executes the
mutate-build-execute loop, `report` renders the evidence, `ci` fails the build
when survivors exceed a threshold.

---

## The problem Mutanchor solves

- **Line coverage lies.** A test suite can touch 100% of the lines and still
  miss every bug that matters. Coverage counts execution, not assertions.
- **Anchor bugs are repetitive.** Solana audits keep finding the same classes —
  missing signer checks, wrong PDA bumps, dropped authority checks. If your
  tests don't catch one, an attacker can use it.
- **Nothing measures the gap.** cargo-mutants targets generic Rust and has zero
  Solana support. Test generators (solify, testship) produce tests; nothing
  verifies the tests would actually fail when the program is broken.

Mutanchor quantifies the gap: kill a mutant, your tests proved something.
Let it survive, and the report shows you the exact line an attacker could stand
on.

---

## How Mutanchor works

1. **Locate.** `init` parses the Anchor IDL and maps every instruction to its
   source file.
2. **Mutate.** Each operator rewrites the Rust AST (syn + quote) to inject one
   bug class. One mutant per fault, no compound mutations.
3. **Build.** Each mutant is compiled with `cargo build-sbf`. A mutant that
   fails to build is classified `build-failed` — itself a signal.
4. **Execute.** The program's own test suite runs against the mutant on
   LiteSVM (anchor-litesvm), in-process, no validator needed.
5. **Classify.** Killed (a test failed), survived (all tests passed), or
   timed out.
6. **Report.** Per-instruction mutation score, surviving mutants with
   operator + file:line + what an attacker could do with each. HTML for
   humans, JSON for CI.

---

## Architecture

```
┌─────────────┐   ┌──────────────────────────────────────────┐   ┌──────────────┐
│ Anchor      │──▶│  mutanchor (Rust CLI)                     │──▶│  LiteSVM      │
│ program     │   │                                          │   │  runner       │
│ (source +   │   │  ┌──────────┐   ┌───────────┐            │   │              │
│  tests)     │   │  │ syn/quote│──▶│ cargo     │──mutant────▶│  │ anchor-      │
│             │   │  │ AST      │   │ build-sbf │  .so        │   │ litesvm      │
│             │   │  │ rewrite  │   │           │             │   │              │
│             │   │  └──────────┘   └───────────┘             │   │  test suite   │
│             │   │                                          │   │  per mutant   │
│             │   │  ┌──────────┐   ┌───────────┐            │   └──────┬───────┘
│             │   │  │ operator │   │ report    │◀─── killed/│          │
│             │   │  │ table    │   │ HTML+JSON │    survived│          ▼
│             │   └──┴──────────┴───┴───────────┘            │   mutation score
└─────────────┘                                              └──────────────
```

### Mutate → build → execute → score

| Step | Tool | Output |
|---|---|---|
| Parse IDL | `mutanchor init` | instruction → source file map |
| Rewrite source | syn + quote | N mutant copies, one fault each |
| Build mutants | `cargo build-sbf` | `.so` per mutant (incremental cache) |
| Run suite | anchor-litesvm | killed / survived / build-failed / timeout |
| Score | report module | mutation score per instruction + overall |

---

## Operators

Each operator models a bug class from the Solana audit-findings corpus:

| Operator | Bug class it models |
|---|---|
| signer check removal | missing signer validation |
| PDA seed swap | wrong seeds, wrong account resolution |
| bump mismatch | using the wrong PDA bump |
| authority check drop | missing owner/authority validation |
| discriminator check removal | accepting wrong instruction discriminators |
| CPI target swap | calling the wrong program |
| close/rent check drop | accounts closed or rent-exempt not validated |
| comparison flip | boundary errors (off-by-one, wrong operator) |

---

## What's real vs pending — the honesty table

| Capability | Status |
|---|---|
| **CLI scaffold** — init / run / report / ci | **Real** — builds, runs, output above |
| **Fresh-clone build** — `cargo check` on clean clone | **Real** — verified |
| **Source rewriting** — syn + quote AST transforms | **Pending** — Phase 1 |
| **Operators** — 8 planned | **Pending** — Phase 1 |
| **LiteSVM runner** — build pipeline + classification | **Pending** — Phase 2 |
| **Report** — HTML viewer + JSON output | **Pending** — Phase 3 |
| **Demo program + published report** | **Pending** — Phase 5 |
| **Demo site** — landing + report viewer, one URL | **Pending** — frontend sourced separately, deployed to Vercel |
| **CI** — build, test, clippy, fmt, cargo audit | **Pending** — Phase 7 |

---

## Tests

No automated tests yet — there are no operators to exercise. The CLI builds
clean and passes clippy-level hygiene gates as they land. Every operator ships
with a unit test on a small fixture program, per the build checklist.

---

## Run it locally

**Prerequisites:** Rust stable (2021 edition).

```bash
git clone https://github.com/subheeksh5599/mutanchor.git
cd mutanchor
cargo build --release
cargo run -- --help
```

---

## Project layout

```
mutanchor/
├── src/
│   └── main.rs            # CLI entry — init / run / report / ci
├── frontend/              # demo site source (landing + report viewer)
├── Cargo.toml
├── LICENSE
└── README.md
```

---

## Tech stack

- **CLI:** Rust, clap (derive)
- **Mutation engine:** syn + quote — deterministic AST rewriting, no LLM in the
  analysis path
- **Execution:** LiteSVM / anchor-litesvm, `cargo build-sbf`
- **Report:** static HTML (Vercel-deployable) + JSON for CI

---

## Roadmap

- Mutation engine: all 8 operators, unit-tested on fixture programs
- LiteSVM runner: incremental build cache, parallel execution, timeout handling
- Report: per-instruction scores, surviving-mutant annotations, CI artifact
- Demo: forked Anchor escrow program with published mutation report
- CI/CD: build, test, clippy, fmt, cargo audit; release workflow

---

## License

MIT — see [LICENSE](LICENSE).
