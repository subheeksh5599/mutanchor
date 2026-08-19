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
- [OK] Real demo report exists: `target/mutanchor/report.json` (75% mutation score, 1 genuine surviving mutant).
- [] Demo report published to the live panel (`/dashboard` shows real data, not the empty state) — needs Vercel deploy (YOU).
- [] Full mutation run on a real program completes with correct verdicts on a capable machine (2-core laptop times out on SBF builds) — needs VPS/CI (YOU).
- [] CI is actually green end-to-end (every job per-run, not the workflow file existing) — needs a push→run observation.
- [OK] No mocks/sample data anywhere; the panel explicitly stays empty until a real run exists.

## 1. Mutation engine correctness

- [OK] All 8 operators implemented (signer, PDA seed, bump, authority, discriminator, CPI target, close/rent, comparison flip).
- [OK] Every operator has a unit test on a fixture program (10 tests green).
- [OK] One-fault-per-mutant holds; equivalent-duplicate mutants dropped.
- [OK] Operator list maps to real Solana audit bug classes.
- [OK] Mutant provenance: operator + file:line recorded.
- [PART] Attribution to instruction handlers (accounts-struct mutants attribute via Context<> mapping) — works on demo, should be tested on a second real program.
- [] Engines tested against ≥3 real Anchor programs (staking, vault, token) — only demo-vault so far.

## 2. LiteSVM runner (execution)

- [OK] Builds each mutant with Solana's official toolchain (`cargo build-sbf`).
- [OK] Executes the program's test suite headlessly on LiteSVM (no validator/cluster).
- [OK] Classifies killed / survived / build-failed / timeout.
- [OK] Incremental single-tree build cache (reuses one scratch tree across mutants).
- [OK] Surfaces real build stderr on failure (env vs code errors distinguishable).
- [] Decouple "build timeout" from "build failed" in verdicts — currently a timed-out build is mislabeled BuildFailed.
- [] Hard per-mutant timeout tuned for realistic SBF builds (default 300s too tight on 2 cores).
- [] Runner handles kill/interruption cleanly (leaves no stuck processes or giant scratch dirs).
- [] `ci` gate verified to actually fail on a >threshold survivor (unit/integration test).

## 3. Report

- [OK] Per-instruction + overall mutation score.
- [OK] Surviving-mutant list with file:line, original/mutated, exploit annotation.
- [OK] HTML self-contained scorecard + `report-full.json` + publishable `report.json`/`dashboard.json` matching the frontend contract.
- [OK] Report panel renders the publishable shape (frontend type contract matches).
- [] Golden-file test for report output (assert stable JSON/HTML structure).

## 4. Testing the tool itself

- [OK] Unit test per operator (10).
- [OK] CLI integration tests (help, init scan, dry-run) (4).
- [] Integration test: plant a real bug, assert a mutant is KILLED (proves the killer path).
- [] Property: every mutant resolves to killed/survived/build-failed/timeout, none hang.
- [] Fresh-clone sweep + full `mutanchor run` E2E asserted in CI (not just unit/CLI).

## 5. Demo program (`demo/demo-vault`)

- [OK] Real Anchor program, compiles via `cargo build-sbf` to a valid `.so`.
- [OK] All 5 LiteSVM tests pass headlessly (deposit, withdraw, over-withdraw revert, pay CPI, unauthorized-close revert).
- [OK] Reads the mutant `.so` via `MUTANCHOR_PROGRAM_SO` (the runner's injection contract).
- [] Published mutation report for the demo program (on CI/VPS) committed/accessible.

## 6. Security & secrets

- [OK] No secrets in repo; machine paths and `.env` gitignored.
- [OK] cargo audit clean on the root lockfile (zero dalek/solana vulnerable entries).
- [OK] No emoji, no hardcoded machine paths, env vars with defaults.
- [] gitleaks pre-commit hook / CI scan (fetch-depth: 0).
- [] Supply-chain: `cargo audit` runs in CI on every push (configured in workflow; needs a green run).
- [PART] Dependency pinning via Cargo.lock — committed at root; demo program lockfile also committed.

## 7. CI/CD

- [OK] GitHub Actions workflow: fmt, clippy -D warnings, build, cargo test, cargo audit.
- [] Demo-job that runs the mutator on demo-vault and uploads the report artifact — file exists, needs a green observed run.
- [] Every job green per-run (verify `gh run view <id>`, not the rollup).
- [] Release workflow (`cargo publish` dry-run / tag) — not yet added.
- [PART] CD: site is deployed and live (HTTP 200 on `/`, `/dashboard`), but it's a manual/CLI deploy — state which; auto-deploy-on-push not proven (YOU: `vercel git connect` + rootDirectory).
- [] Rollback path documented (redeploy previous commit works).

## 8. Repo & docs polish (readme-repo-polish standard)

- [OK] README honesty table: engine/operators/runner/report/CI marked Real; demo program honestly "in progress/published"; roadmap checkboxes.
- [OK] README starts with the clone command; see-it-in-one-command shows REAL `--help` output.
- [OK] CHANGELOG present.
- [OK] No build artifacts or nested targets committed (`.gitignore` covers `**/target` + generated reports).
- [] Repo description + topics (`gh repo edit subheeksh5599/mutanchor --description ... --add-topic ...`).
- [] shields.io badge row including a Tests/CI badge linked to real green runs + the live URL.
- [] Real screenshots of the CLI output / report panel (headless capture, pixel-variance-checked) in a Screenshots section.
- [] `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` — missing (checklist Phase 6 / production standard).
- [] Cargo.toml package metadata: `repository`, `homepage`, `readme`, keywords, categories (needed for a real publish).
- [] Operators table + "why line coverage lies" doc in README (currently only mentioned).
- [] `.env.example` present and in sync with what the code actually reads (the only required env is `MUTANCHOR_PROGRAM_SO`, for the runner's test runs — document it).

## 9. Ops & runtime

- [OK] No server/daemon to run (one-shot CLI) — ops surface is minimal.
- [OK] Health: the tool validates its inputs and fails with clear errors (e.g. "cannot read src/lib.rs").
- [] `--help` / error paths tested for every subcommand (integration coverage).
- [] Performance: benchmark large-program runs; document expected runtime vs mutant count (honest expectations for users).

## 10. Legal / framework

- [OK] MIT license present.
- [] README honest-limitations section: what a dev tool can/can't claim (mutation score ≠ audit; "dev-tool" framing per the grant).
- [] Publish metadata finalized (see §8) before any `cargo publish`.

---

## Score (production-audit bands)

Current honest estimate: **~70/100 — Launchable With Caveats.** Headroom was lost
on: full-execution not demonstrable on this laptop (timeout misclassification),
no green observed CI run, panel not publishing live data, missing repo polish
(CONTRIBUTING/CODE_OF_CONDUCT/metadata/badges/topics), golden-file + kill-path
tests. Cap at 84 because the launch-critical path (fresh clone → build → test →
run → report) is verified, but full E2E execution on real hardware is not.

Once: (a) a CI run goes green per-job, (b) the demo job produces a report on CI,
(c) the panel publishes real data, and (d) the repo-polish set lands → **85+**.

## Top items to do first (picks itself from blast radius)

1. Point a CI/VPS run at the demo program and watch a full `mutanchor run` complete with correct verdicts (fixes the timeout-mislabel and gives real proof).
2. Publish the demo report to the panel (needs Vercel deploy — YOU).
3. Add the cheap code/CI items: kill-path integration test, golden-file report test, `ci` gate test, runner timeout-distinction.
4. Repo polish: description/topics, connected badges, screenshots, CONTRIBUTING/CODE_OF_CONDUCT, Cargo.toml publish metadata.
5. Add the demo mutation-job to CI and verify it green.

_Autogenerated audit doc — keep in the repo as the durable to-do (each item
independent and tickable, per the web3-production-checklist close-out rule)._
