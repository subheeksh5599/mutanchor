<div align="center">

# Mutanchor

[![Live demo](https://img.shields.io/badge/live-mutanchor.vercel.app-D9FF00)](https://mutanchor.vercel.app)
[![License: MIT](https://img.shields.io/badge/license-MIT-14151a.svg)](LICENSE)
![Language](https://img.shields.io/badge/Rust%202021-14151a)
![Target](https://img.shields.io/badge/Solana%20Anchor-14151a)
![Status](https://img.shields.io/badge/status-early%20scaffold-9ca3af)

### Line coverage lies. Mutanchor proves which bugs your tests would miss.

Mutation testing for Solana Anchor programs. Mutanchor rewrites Anchor source at
the AST level to inject real bug classes — missing signer checks, PDA bump
mismatches, dropped authority checks, discriminator removal, boundary flips,
CPI target swaps — then builds each mutant with `cargo build-sbf` and runs the
program's test suite against it on LiteSVM. A mutant that survives means the
tests would miss that bug in production.

cargo-mutants has zero Solana support. No Solana mutation tool ships. Mutanchor
is that gap.

**[ Live demo ↗ ](https://mutanchor.vercel.app)** · **[ Report panel ↗ ](https://mutanchor.vercel.app/dashboard)** · **[ Source ↗ ](https://github.com/subheeksh5599/mutanchor)** · **[ Architecture ↓ ](#architecture)** · **[ Run it locally ↓ ](#run-it-locally)**

Built for the Solana ecosystem. MIT licensed.

</div>

---

## Table of contents

- [See it in one command](#-see-it-in-one-command)
- [The problem Mutanchor solves](#the-problem-mutanchor-solves)
- [How Mutanchor works](#how-mutanchor-works)
- [Architecture](#architecture)
- [Operators](#operators)
- [How it uses Solana](#how-it-uses-solana)
- [Engineering decisions & the hard problems](#engineering-decisions--the-hard-problems)
- [What's real vs pending — the honesty table](#whats-real-vs-pending--the-honesty-table)
- [Tests](#tests)
- [Run it locally](#run-it-locally)
- [Deploy](#deploy)
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

That output is real — build the crate and run it. `init` locates what to
mutate, `run` executes the mutate-build-execute loop, `report` renders the
evidence, `ci` fails the build when survivors exceed a threshold.

---

## The problem Mutanchor solves

- **Line coverage lies.** Coverage counts execution, not assertions. A suite
  can touch every line and still miss every bug that matters.
- **Anchor bugs are repetitive.** Solana audits keep finding the same classes:
  missing signer checks, wrong PDA bumps, dropped authority checks. If your
  tests do not catch one, an attacker can use it.
- **Nothing measures the gap.** cargo-mutants targets generic Rust with zero
  Solana support. Test generators (solify, testship) produce tests; nothing
  verifies the tests would actually fail when the program is broken.

Existing metrics are execution counts. Mutanchor measures faults detected:
kill a mutant, your tests proved something; let it survive, the report shows
the exact line an attacker could stand on.

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

## How it uses Solana

**Builds.** Every mutant is compiled with `cargo build-sbf` — the real Solana
BPF toolchain — producing a loadable `.so` per mutant.

**Executes.** Mutants run on LiteSVM, the in-process Solana runtime used by
the Anchor test harness (anchor-litesvm). Instruction-level test suites run
against each mutated build with no validator, no cluster, no wait.

**Parses.** The Anchor IDL (the program's own interface definition) drives
`init`: instructions are mapped to their source files, so mutants land exactly
where the audit corpus says bugs live.

---

## Engineering decisions & the hard problems

- **Deterministic mutation, no LLM in the analysis path.** Operators are AST
  rewrites (syn + quote) — the same input always produces the same mutants.
  AI accelerates the build of the tool; it never judges the verdict.

- **One fault per mutant.** Compound mutations confound results. Every mutant
  carries exactly one injected bug, so every survivor is attributable to one
  operator at one file:line.

- **Equivalent-mutant dedup.** Mutants that change no behavior are detected
  and dropped — they would otherwise drag the score down without meaning.

- **Build-failed is a signal, not a crash.** A mutant that fails to compile
  classifies as `build-failed` and the run continues. Hanging mutants hit a
  timeout, never an infinite wait.

- **In-process execution.** LiteSVM removes the validator bootstrap — the
  whole mutate-build-execute loop is one process, which is what makes mutation
  testing tractable for Anchor at all.

- **No secrets, no hardcoded paths.** The repo carries no keys and no
  absolute paths. Configuration arrives with the runner: env vars with
  defaults only.

---

## What's real vs pending — the honesty table

| Capability | Status |
|---|---|
| **CLI scaffold** — init / run / report / ci | **Real** — builds, runs, output above |
| **Fresh-clone build** — `cargo check` on clean clone | **Real** — verified |
| **Demo site** — landing + report panel, one URL | **Real** — live at [mutanchor.vercel.app](https://mutanchor.vercel.app) |
| **Source rewriting** — syn + quote AST transforms | **Pending** — Phase 1 |
| **Operators** — 8 planned | **Pending** — Phase 1 |
| **LiteSVM runner** — build pipeline + classification | **Pending** — Phase 2 |
| **Report** — HTML viewer + JSON output | **Pending** — Phase 3 |
| **Demo program + published report** | **Pending** — Phase 5 |
| **CI** — build, test, clippy, fmt, cargo audit | **Pending** — Phase 7 |

---

## Tests

No automated tests yet — there are no operators to exercise. The CLI builds
clean. Every operator ships with a unit test on a small fixture program, per
the build checklist. Nothing on this site or in this README is sample data;
the report panel stays empty until a real `mutanchor run` exists.

---

## Run it locally

**Prerequisites:** Rust stable (2021 edition), Node.js 18+ (demo site only).

```bash
git clone https://github.com/subheeksh5599/mutanchor.git
cd mutanchor

# Build the CLI
cargo build --release

# Run it
cargo run -- --help

# Run the demo site locally
cd frontend && npm install && npm run dev   # :5173
```

---

## Deploy

| | |
|---|---|
| **Demo site** | **[mutanchor.vercel.app](https://mutanchor.vercel.app)** — Vercel, landing at `/`, report panel at `/dashboard` |
| **Source** | **[github.com/subheeksh5599/mutanchor](https://github.com/subheeksh5599/mutanchor)** |

Report contract: `mutanchor run` emits `report.json`; copy it to
`frontend/public/` and redeploy. The panel renders the real output and stays
empty until that file exists.

---

## Project layout

```
mutanchor/
├── src/
│   └── main.rs            # CLI entry — init / run / report / ci
├── frontend/              # demo site (Vite + React + Tailwind v4)
│   └── public/            # report.json lands here before deploy
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
- **Demo site:** Vite, React, Tailwind v4 — self-hosted fonts, light mode

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
