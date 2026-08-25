<div align="center">

# Mutanchor

[![Live demo](https://img.shields.io/badge/live-mutanchor.vercel.app-D9FF00)](https://mutanchor.vercel.app)
[![License: MIT](https://img.shields.io/badge/license-MIT-14151a.svg)](LICENSE)
![Language](https://img.shields.io/badge/Rust%202021-14151a)
![Target](https://img.shields.io/badge/Solana%20Anchor-14151a)
![Type](https://img.shields.io/badge/type-standalone%20CLI-14151a)
![Status](https://img.shields.io/badge/status-working%20engine-4caf50)

### Line coverage lies. Mutanchor proves which bugs your tests would miss.

Mutanchor is a testing tool for Solana programs built with Anchor. It makes
small, deliberate changes to a program's source code, one at a time, then runs
the program's test suite against each changed version. If the tests catch the
change, it was a good test. If they don't, that change is a bug your tests
would miss in production — and the report shows you exactly where it is and
what an attacker could do with it.

It is a CLI, not an AI tool and not a skill: every change comes from a
fixed rule set, so results are deterministic and reproducible. AI is used to
build the tool, never to judge the results.

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
  report  Emit the mutation report
  ci      CI mode: fail when surviving mutants exceed a threshold
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

That output is real — build the crate and run it. `init` finds what to change,
`run` does the changing and testing, `report` produces the scorecard, `ci`
fails the build when too many bugs slip through.

---

## The problem Mutanchor solves

- **Line coverage lies.** Coverage counts which lines ran, not whether the
  checks are real. A suite can touch every line and still miss every bug that
  matters.
- **Anchor bugs are repetitive.** Solana audits keep finding the same classes:
  missing signer checks, wrong PDA bumps, dropped authority checks. If your
  tests do not catch one, an attacker can use it.
- **Nothing measures the gap.** Existing tools generate tests for you, but
  nothing checks whether those tests would actually fail when the program is
  broken.

Mutanchor measures the gap directly: kill a mutant, your tests proved
something. Let it survive, and the report shows the exact line an attacker
could stand on.

---

## How Mutanchor works

1. **Locate.** The tool reads the Anchor program and maps every instruction to
   its source file.
2. **Mutate.** It makes one small, deliberate change to the source code — the
   kind of mistake Solana audits keep finding. One change per version, never
   two at once.
3. **Build.** Each changed version is compiled with Solana's official build
   toolchain. One that fails to compile is recorded as build-failed, which is
   itself useful information.
4. **Execute.** The program's own test suite runs against each changed version
   on LiteSVM, a fast in-memory Solana runtime — no network, no waiting.
5. **Classify.** Killed (a test failed), survived (all tests passed), or timed
   out.
6. **Report.** A mutation score per instruction and overall, plus a list of
   every surviving change with its file and line and what an attacker could do
   with it.

---

## Architecture

```
┌─────────────┐   ┌──────────────────────────────────────────┐   ┌──────────────┐
│ Anchor      │──▶│  mutanchor (Rust CLI)                     │──▶│  LiteSVM      │
│ program     │   │                                          │   │  runtime      │
│ (source +   │   │  ┌──────────────┐   ┌──────────────┐     │   │              │
│  tests)     │   │  │ source       │──▶│ build        │──change─▶│  runs the    │
│             │   │  │ rewriter     │   │ (Solana      │  .so  │   │  test suite  │
│             │   │  │ (one change  │   │  toolchain)  │      │   │  per version │
│             │   │  │  per mutant) │   │              │      │   └──────┬───────┘
│             │   │  └──────────────┘   └──────────────┘      │          │
│             │   │  ┌──────────────┐   ┌──────────────┐      │          ▼
│             │   │  │ operator     │   │ report       │◀─────│  mutation score
│             │   │  │ table        │   │ scorecard    │      │
│             │   └──┴──────────────┴───┴──────────────┘      └──────────────
└─────────────┘
```

### Change → build → test → score

| Step | How | What comes out |
|---|---|---|
| Read the program | `mutanchor init` | instruction → source file map |
| Make one change | source rewriter | one changed version per fault |
| Build the change | Solana build toolchain | a runnable program per version |
| Run the tests | LiteSVM | killed / survived / build-failed / timeout |
| Score | report | mutation score per instruction + overall |

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

**Builds.** Every changed version is compiled with Solana's official build
toolchain, the same one real Anchor programs use.

**Executes.** Versions run on LiteSVM, the in-memory Solana runtime used by the
Anchor test harness — instruction-level tests, no validator, no cluster, no
wait.

**Reads.** The Anchor program's own interface file drives the tool: every
instruction is mapped to its source file, so changes land exactly where the
audit corpus says bugs live.

---

## Engineering decisions & the hard problems

- **A CLI, not an AI tool.** Mutanchor is a standalone command-line tool, not
  a skill and not an agent. The changes come from a fixed set of rewriting
  rules — same input, same mutants, every time. AI tools speed up building the
  tool itself; they never decide whether a change was caught.

- **One fault per mutant.** Two changes at once make results impossible to
  read. Every mutant carries exactly one injected bug, so every survivor is
  attributable to one rule at one file:line.

- **Meaningless mutants are dropped.** A change that cannot affect behavior
  would drag the score down without meaning, so it is detected and removed.

- **Build-failed is a signal, not a crash.** A mutant that does not compile is
  recorded and the run continues. A mutant that hangs hits a timeout — the run
  never waits forever.

- **Fast execution is the whole point.** LiteSVM runs everything in one
  process, which is what makes this kind of testing practical for Solana at
  all.

- **No secrets, no hardcoded paths.** The repo carries no keys and no machine
  paths. Configuration arrives with the runner: environment variables with
  defaults only.

---

## What's real vs pending — the honesty table

| Capability | Status |
|---|---|
| **CLI scaffold** — init / run / report / ci | **Real** — builds, runs, output above |
| **Fresh-clone build** — `cargo check` on clean clone | **Real** — verified |
| **Demo site** — docs + live report panel, one URL | **Real** — docs-first site (install/quickstart/operators/CLI/report) at [mutanchor.vercel.app](https://mutanchor.vercel.app), live panel at [/dashboard](https://mutanchor.vercel.app/dashboard) |
| **Source rewriting** — one change per mutant | **Real** — engine implemented, unit-tested |
| **Operators** — 8 implemented | **Real** — all 8 implemented, unit-tested on fixture programs |
| **LiteSVM runner** — build pipeline + classification | **Real** — implemented; run against the demo program (see below) |
| **Report** — scorecard per instruction | **Real** — HTML + JSON + dashboard.json generated by the CLI |
| **Demo program + published report** | **Real** — live run on `demo/demo-vault`: **76.9% mutation score (9 killed, 3 survived, 1 build-failed, 0 timeout)**, published at [mutanchor.vercel.app/dashboard](https://mutanchor.vercel.app/dashboard) (raw: `/report.json`) |
| **CI** — build, test, clippy, fmt, cargo audit | **Real** — GitHub Actions workflow in this repo |

---

## Tests

The mutation engine ships with a unit test per operator, each running against a
small fixture program (`demo/fixture`), plus property-style checks that every
mutant is exactly one fault and that generic/lifetime syntax is never mutated:

```bash
cargo test
```

Currently 30 tests are green: the engine's per-operator unit tests, the report
contract (publishable JSON shape, HTML, empty state, timeout-exclusion score
rule), the CI gate, the CLI integration suite, and the demo program's own
Anchor/LiteSVM suite. Nothing on this site or in this README is sample data;
the report panel renders only the output of a real `mutanchor run`.

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

## Environment

See `.env.example` for the full list. Mutanchor's CLI needs no environment
variables — `run` sets the one it uses itself:

| Variable | Required | Used by | Purpose |
|---|---|---|---|
| `MUTANCHOR_PROGRAM_SO` | only for `cargo test` on a program | `mutanchor run` (set automatically) + the demo program's LiteSVM test suite | Path to the compiled `program.so` the suite loads into LiteSVM. `mutanchor run` points each mutant's test run at that mutant's freshly built `.so`. You normally never set this by hand. |

There are no secrets in this project. Nothing is read from the environment that
isn't documented here, and the repo carries no keys.

---

## Deploy

| | |
|---|---|
| **Demo site** | **[mutanchor.vercel.app](https://mutanchor.vercel.app)** — Vercel, landing at `/`, report panel at `/dashboard` |
| **Source** | **[github.com/subheeksh5599/mutanchor](https://github.com/subheeksh5599/mutanchor)** |

The report panel shows the real output of `mutanchor run`. Run the tool on a
program, publish the resulting report, and the panel renders it. Until then it
stays empty.

```bash
# run the mutator on a program, then publish the report to the site
mutanchor run demo/demo-vault --out target/mutanchor
scripts/publish-report.sh target/mutanchor/report.json
cd frontend && npm run build && vercel --prod
```

`scripts/publish-report.sh` copies the CLI's `report.json` into
`frontend/public/` so the `/dashboard` panel renders it at the site root.

---

## Project layout

```
mutanchor/
├── src/
│   ├── main.rs            # CLI entry — init / run / report / ci
│   ├── engine.rs          # orchestration: scan → mutate → run → report
│   ├── ops.rs             # the 8 mutation operators (deterministic rules)
│   ├── init.rs            # instruction → source-file mapping
│   ├── runner.rs          # build each mutant + execute on LiteSVM
│   ├── report.rs          # HTML + JSON report rendering
│   └── model.rs           # shared data model (Mutant, Verdict, Report)
├── demo/
│   ├── fixture/           # tiny fixture program for operator unit tests
│   └── demo-vault/        # demo Anchor program + LiteSVM test suite
├── frontend/              # demo site (Vite + React + Tailwind v4)
│   └── public/            # reports land here before publishing
├── .github/workflows/     # CI: build, test, clippy, fmt, audit, demo-job
├── Cargo.toml
├── LICENSE
└── README.md
```

---

## Tech stack

- **CLI:** Rust
- **Mutation engine:** source rewriting with a fixed rule set — deterministic,
  no AI in the analysis path
- **Execution:** LiteSVM (anchor-litesvm) + Solana's official build toolchain
- **Report:** generated by the CLI, viewable on the demo site
- **Demo site:** Vite, React, Tailwind v4 — self-hosted fonts, light mode

---

## Roadmap

- [x] Mutation engine: all 8 operators, unit-tested on fixture programs
- [x] LiteSVM runner: build each mutant + run the suite on LiteSVM, k/s/bf/t classification
- [x] Report: per-instruction scores, surviving-mutant annotations with exploit sketches, JSON + HTML + dashboard artifact
- [x] Demo: `demo/demo-vault` (real Anchor program, 10-test LiteSVM suite) with a **live published mutation report** — [mutanchor.vercel.app/dashboard](https://mutanchor.vercel.app/dashboard), 76.9% score (run: `cargo run --release -- run demo/demo-vault`)
- [x] CI/CD: build, test, clippy, fmt, cargo audit; demo-job publishes the report artifact (workflow in `.github/workflows/ci.yml`)

---

## License

MIT — see [LICENSE](LICENSE).
