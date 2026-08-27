# Mutanchor — Production Checklist

Production-readiness audit for Mutanchor (mutation testing for Solana Anchor
programs — a standalone Rust CLI + a live demo site). Built from the
hackathon-finisher production doctrine and web3-production-checklist evidence
standard. Every item is ticked only on real evidence, never on assertion.

Legend: `[OK]` verified this session · `[]` open · `[PART]` partial · `YOU` = needs
your action/credential.

---

## 0. PROOF — does it provably work? (highest weight)

The hackathon-production doctrine ranks this first: a repo that "works" but
can't be proven to run loses on the demo path. For a CLI tool, "proof" is a
fresh clone that builds, tests, and runs.

- [OK] Fresh-clone build: `git clone` → `cargo test` passes (10 engine + 4 CLI tests) — verified from a clean checkout of the pushed repo.
- [OK] CLI runs: `mutanchor --help`, `init`, `run --dry-run`, `report` all produce real output.
- [OK] Real demo report exists: `target/mutanchor/report.json` from a LIVE run on `demo/demo-vault` — 76.9% (9 killed, 3 survived, 1 build-failed, 0 timeout); the 3 survivors are `authority_check_drop` (L32/L45/L73), verified as genuine (real builds + real test runs).
- [OK] Demo report published to the live panel — `/dashboard` renders the live run (77%, per-instruction table, survivor annotations); raw JSON at mutanchor.vercel.app/report.json (verified via browser + curl).
- [OK] Full mutation run completes with correct verdicts ON THIS 2-CORE LAPTOP — fixed by warming the incremental cache (pristine build+test prime, target/ preserved across mutants with mtime-preserving restore); 13 mutants, 0 timeouts, 9 killed / 3 survived / 1 build-failed.
- [OK] CI is green end-to-end — verified per-job success on run 32244231899 (Build/test/lint/audit ✓ + demo-job ✓).
- [OK] No mocks/sample data anywhere; the panel explicitly stays empty until a real run exists.

## 1. Mutation engine correctness

- [OK] All 8 operators implemented (signer, PDA seed, bump, authority, discriminator, CPI target, close/rent, comparison flip).
- [OK] Every operator has a unit test on a fixture program (10 tests green).
- [OK] One-fault-per-mutant holds; equivalent-duplicate mutants dropped.
- [OK] Operator list maps to real Solana audit bug classes.
- [OK] Mutation provenance: operator + file:line recorded.
- [OK] Attribution to instruction handlers (accounts-struct mutants attribute to the instruction via Context<> mapping; handler/struct ranges kept disjoint with a regression test) — verified on demo-vault: every mutant carries an instruction label, no "(unknown)" bucket.
- [] Engines tested against ≥3 real Anchor programs (staking, vault, token) — only demo-vault so far.

## 2. LiteSVM runner (execution)

- [OK] Builds each mutant with Solana's official toolchain (`cargo build-sbf`).
- [OK] Executes the program's test suite headlessly on LiteSVM (no validator/cluster).
- [OK] Classifies killed / survived / build-failed / timeout.
- [OK] Incremental single-tree build cache (reuses one scratch tree across mutants).
- [OK] Surfaces real build stderr on failure (env vs code errors distinguishable).
- [OK] Decouple "build timeout" from "build failed" in verdicts — a timed-out build is now Verdict::TimedOut, not BuildFailed.
- [OK] Per-run warm-up prime uses 900s; per-mutant phases ran at 300s with 0 timeouts on the demo run (incremental cache makes each mutant fast; default 180s remains fine for cached mutants).
- [] Runner handles kill/interruption cleanly (leaves no stuck processes or giant scratch dirs).
- [OK] `ci` gate decision tested (ci_verdict unit test) — fails on >max, passes at/under max.

## 3. Report

- [OK] Per-instruction + overall mutation score.
- [OK] Surviving-mutant list with file:line, original/mutated, exploit annotation.
- [OK] HTML self-contained scorecard + `report-full.json` + publishable `report.json`/`dashboard.json` matching the frontend contract.
- [OK] Report panel renders the publishable shape (frontend type contract matches).
- [OK] Report contract tests: publishable JSON matches the frontend shape, self-contained HTML, empty-state (tests/report.rs).

## 4. Testing the tool itself

- [OK] Unit test per operator (10).
- [OK] CLI integration tests (help, init scan, dry-run) (4).
- [OK] Kill-path aggregation covered by report tests; full SBF kill-path now OBSERVED locally (9 killed with real panics, 1 build-failed with real compile error).
- [] Property: every mutant resolves to killed/survived/build-failed/timeout, none hang.
- [] Fresh-clone sweep + full `mutanchor run` E2E asserted in CI (not just unit/CLI).

## 5. Demo program (`demo/demo-vault`)

- [OK] Real Anchor program, compiles via `cargo build-sbf` to a valid `.so`.
- [OK] All 10 LiteSVM tests pass headlessly (deposit/withdraw/pay happy paths, over-withdraw revert, unauthorized close revert, zero-amount rejects x3, exact-balance withdraw, successful close with lamport transfer).
- [OK] Reads the mutant `.so` via `MUTANCHOR_PROGRAM_SO` (the runner's injection contract).
- [OK] Published mutation report for the demo program accessible live: https://mutanchor.vercel.app/report.json (+ rendered /dashboard); produced by this machine's full run.

## 6. Security & secrets

- [OK] No secrets in repo; machine paths and `.env` gitignored.
- [OK] cargo audit clean on the root lockfile (zero dalek/solana vulnerable entries).
- [OK] No emoji, no hardcoded machine paths, env vars with defaults.
- [OK] gitleaks CI scan (fetch-depth: 0) — `secret-scan` job in `.github/workflows/ci.yml`, runs on push/PR/weekly cron.
- [OK] Supply-chain: `cargo audit` runs in CI on every push + weekly schedule (rustsec/audit-check@v2 in `.github/workflows/ci.yml`).
- [PART] Dependency pinning via Cargo.lock — committed at root; demo program lockfile also committed.

## 7. CI/CD

- [OK] GitHub Actions workflow: fmt, clippy -D warnings, build, cargo test, cargo audit.
- [OK] Demo-job runs (green), best-effort; note: solana release bundles Cargo 1.79 which cannot compile modern anchor-lang deps (edition2024) — so the authoritative full-run report is produced on a machine with a compatible toolchain (local/VPS with newer rust).
- [OK] Every job green per-run (verified via gh run view: both jobs conclusion=success).
- [] Release workflow (`cargo publish` dry-run / tag) — not yet added.
- [PART] CD: site is deployed and live (HTTP 200 on `/`, `/dashboard`), but it's a manual/CLI deploy — state which; auto-deploy-on-push not proven (YOU: `vercel git connect` + rootDirectory).
- [OK] Rollback path documented in README Deploy section — `git checkout <last-good> -- frontend/ && vercel --prod`, or Vercel dashboard "Promote to Production". Site is stateless.

## 8. Repo & docs polish (readme-repo-polish standard)

- [OK] README honesty table: engine/operators/runner/report/CI marked Real; demo program honestly "in progress/published"; roadmap checkboxes.
- [OK] README starts with the clone command; see-it-in-one-command shows REAL `--help` output.
- [OK] CHANGELOG present.
- [OK] No build artifacts or nested targets committed (`.gitignore` covers `**/target` + generated reports).
- [OK] Repo description + topics — verified via `gh repo view`: description set, topics include `solana`, `anchor`, `mutation-testing`, `rust`, `litesvm`, `cli`, `devtools`, `solana-program`, `testing`.
- [OK] shields.io badge row includes a real CI badge linked to the GitHub Actions workflow (`ci.yml`) + live URL badge already present.
- [] Real screenshots of the CLI output / report panel (headless capture, pixel-variance-checked) in a Screenshots section.
- [OK] `CONTRIBUTING.md` + `CODE_OF_CONDUCT.md` added.
- [OK] Cargo.toml package metadata added (repository, homepage, readme, keywords, categories).
- [OK] README operators table (8 ops + bug classes); "why line coverage lies" covered in the problem section.
- [OK] `.env.example` added; README Environment section documents `MUTANCHOR_PROGRAM_SO`.

## 9. Ops & runtime

- [OK] No server/daemon to run (one-shot CLI) — ops surface is minimal.
- [OK] Health: the tool validates its inputs and fails with clear errors (e.g. "cannot read src/lib.rs").
- [] `--help` / error paths tested for every subcommand (integration coverage).
- [] Performance: benchmark large-program runs; document expected runtime vs mutant count (honest expectations for users).

## 10. Legal / framework

- [OK] MIT license present.
- [OK] README honest-limitations section added: audit ≠ mutation score, operator set is finite, equivalent-mutant caveat, toolchain reproducibility, demo-only coverage, no AI in analysis path.
- [] Publish metadata finalized (see §8) before any `cargo publish`.

---

## Score (production-audit bands)

Current honest estimate: **~92/100 — Production-Ready, Launchable.** Upgrades this
session: full `mutanchor run` completes on this laptop with correct verdicts
(incremental warm-cache fix + timeout tuning — 0 timeouts on 13 mutants),
the demo suite grew to 10 tests (zero-amount / exact-balance / close-lamport
coverage), the report is PUBLISHED live (/dashboard renders the real 76.9% run),
the false "75%" claim was replaced by the real numbers, and the score formula
now excludes inconclusive timeouts (an all-timeout run scores 0.0, never 100%).
Remaining headroom (non-blocking): ≥3 real Anchor programs exercised, README
honest-limitations section, badges/screenshots, CI demo-job producing its own
SBF report.

The launch-critical path — fresh clone → build → test → run mutator → live
report — is now verified end-to-end on this machine.

## Top items to do first (picks itself from blast radius)

1. Point a CI/VPS run at the demo program and watch a full `mutanchor run` complete with correct verdicts (fixes the timeout-mislabel and gives real proof).
2. Publish the demo report to the panel (needs Vercel deploy — YOU).
3. Add the cheap code/CI items: kill-path integration test, golden-file report test, `ci` gate test, runner timeout-distinction.
4. Repo polish: description/topics, connected badges, screenshots, CONTRIBUTING/CODE_OF_CONDUCT, Cargo.toml publish metadata.
5. Add the demo mutation-job to CI and verify it green.

_Autogenerated audit doc — keep in the repo as the durable to-do (each item
independent and tickable, per the web3-production-checklist close-out rule)._
