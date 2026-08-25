import { Link } from "react-router-dom";
import { Nav } from "../components/Nav";
import { Footer } from "../components/Footer";

const operators: Array<[string, string, string]> = [
  ["signer check removal", "missing signer validation", "Signer-flavored account accepts a non-signer"],
  ["PDA seed swap", "wrong seeds, wrong account resolution", "derives the wrong address, funds move to the wrong vault"],
  ["bump mismatch", "using the wrong PDA bump", "canonical address unresolved, account not found"],
  ["authority check drop", "missing owner or authority validation", "non-authorised caller runs restricted logic"],
  ["discriminator check removal", "accepting wrong instruction discriminators", "malformed instruction data passes as valid"],
  ["CPI target swap", "calling the wrong program", "transfer/close routed to an attacker-controlled program"],
  ["close or rent check drop", "accounts closed or not rent exempt, unvalidated", "lamports reclaimed, account data lost"],
  ["comparison flip", "boundary errors, off by one, wrong operator", "zero-amount or over-balance operations slip through"],
];

const cli: Array<[string, string]> = [
  ["mutanchor init [PROGRAM]", "Parse the Anchor program, locate instruction handlers and their source files, and map each Accounts struct to the instruction that uses it."],
  ["mutanchor run [PROGRAM]", "Mutate, build every mutant with cargo build-sbf, run the program's real test suite against it on LiteSVM, and classify the outcome (killed / survived / build-failed / timeout)."],
  ["mutanchor report [REPORT]", "Render a saved run as self-contained HTML (and JSON) — no network assets."],
  ["mutanchor ci [PROGRAM]", "CI mode: fail with exit code 1 when surviving mutants exceed --max-survivors."],
];

const flags: Array<[string, string]> = [
  ["run --dry-run", "Emit the mutation set without building or executing."],
  ["run --skip <OPERATOR>", "Exclude an operator from the run."],
  ["run --timeout <SECS>", "Per-mutant build/test timeout (default 180s)."],
  ["run --out <DIR>", "Output directory for report files (default target/mutanchor)."],
  ["ci --max-survivors <N>", "Fail the build when survivors exceed N (default 0)."],
  ["ci --survivors-only", "Only survivors count towards the threshold."],
];

export default function Docs() {
  return (
    <div className="bg-bone">
      <Nav />
      <main>
        <section id="what" className="mx-auto max-w-3xl px-5 pb-16 pt-14">
          <p className="mono-label">Solana Anchor</p>
          <h1 className="display mt-3 text-[clamp(2.2rem,5vw,3.6rem)] leading-[1.05]">
            Mutation testing for Solana Anchor programs
          </h1>
          <p className="mt-6 text-sm leading-relaxed text-ink/70">
            Mutanchor rewrites Anchor program source at the AST level to inject
            real bug classes, builds each mutant with{" "}
            <code className="font-mono text-[0.8em]">cargo build-sbf</code>,
            runs the program's own test suite against it headlessly on LiteSVM,
            and reports which mutants survive. A survivor is a bug your tests
            would miss in production.
          </p>
          <p className="mt-4 text-sm leading-relaxed text-ink/70">
            It is a local CLI. Every change comes from a fixed rule set, not an
            AI: same program, same mutants, deterministic output. No cluster is
            involved — execution is fully in-process.
          </p>
        </section>

        <section id="install" className="border-t border-ink/15">
          <div className="mx-auto max-w-3xl px-5 py-14">
            <p className="mono-label">Install</p>
            <h2 className="display mt-3 text-3xl">BUILD THE CLI</h2>
            <pre className="mt-6 overflow-x-auto border-2 border-ink bg-pitch p-5 text-[0.8rem] leading-relaxed text-bone">
              <code>{`git clone https://github.com/subheeksh5599/mutanchor.git
cd mutanchor
cargo build --release
# binary at target/release/mutanchor`}</code>
            </pre>
            <p className="mt-4 text-sm leading-relaxed text-ink/70">
              Requires a Rust toolchain (2021 edition or newer) and the Solana
              build toolchain (<code className="font-mono text-[0.8em]">cargo build-sbf</code>).
              The demo site is only a viewer for published reports; the tool
              itself runs locally.
            </p>
          </div>
        </section>

        <section id="quickstart" className="border-t border-ink/15">
          <div className="mx-auto max-w-3xl px-5 py-14">
            <p className="mono-label">Quickstart</p>
            <h2 className="display mt-3 text-3xl">INIT — RUN — REPORT</h2>

            <pre className="mt-6 overflow-x-auto border-2 border-ink bg-pitch p-5 text-[0.8rem] leading-relaxed text-bone">
              <code>{`$ mutanchor init demo/demo-vault
program dir: demo/demo-vault
instructions (5):
   24  create                   src/lib.rs
   30  deposit                  src/lib.rs
   42  withdraw                 src/lib.rs
   54  pay                      src/lib.rs
   72  close                    src/lib.rs`}</code>
            </pre>

            <pre className="mt-4 overflow-x-auto border-2 border-ink bg-pitch p-5 text-[0.8rem] leading-relaxed text-bone">
              <code>{`$ mutanchor run demo/demo-vault
generated 13 mutants (0 equivalent/duplicate dropped) across 1 file(s)
priming incremental build cache (pristine build + test)…
cache primed
[1/13] authority_check_drop at src/lib.rs:32
[2/13] authority_check_drop at src/lib.rs:45
...
mutation score: 76.9% (9 killed, 3 survived, 1 build-failed, 0 timeout)
report.json:  target/mutanchor/report.json
report.html:  target/mutanchor/report.html
dashboard.json: target/mutanchor/dashboard.json`}</code>
            </pre>

            <pre className="mt-4 overflow-x-auto border-2 border-ink bg-pitch p-5 text-[0.8rem] leading-relaxed text-bone">
              <code>{`$ mutanchor report target/mutanchor/report.json --html report.html
$ mutanchor ci demo/demo-vault --max-survivors 0
surviving mutants: 3 (max allowed: 0)
error: CI gate: 3 surviving mutants exceeds max of 0`}</code>
            </pre>

            <p className="mt-4 text-sm leading-relaxed text-ink/70">
              The in-repo demo program (<code className="font-mono text-[0.8em]">demo/demo-vault</code>,
              5 instructions, 10-test LiteSVM suite) is the reference run:{" "}
              <Link to="/dashboard" className="underline decoration-ink/40 underline-offset-4 hover:decoration-ink">
                the live report
              </Link>
              .
            </p>
          </div>
        </section>

        <section id="operators" className="border-t border-ink/15">
          <div className="mx-auto max-w-3xl px-5 py-14">
            <p className="mono-label">Operators</p>
            <h2 className="display mt-3 text-3xl">EIGHT BUG CLASSES</h2>
            <p className="mt-4 text-sm leading-relaxed text-ink/70">
              Each operator models a recurring class from real Solana audit
              findings. One operator fires per location; a mutant is exactly
              one fault.
            </p>
            <div className="mt-6 overflow-x-auto border-2 border-ink">
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b-2 border-ink">
                    <th className="mono-label px-4 py-3">Operator</th>
                    <th className="mono-label px-4 py-3">Models</th>
                    <th className="mono-label px-4 py-3">What an attacker gains</th>
                  </tr>
                </thead>
                <tbody>
                  {operators.map(([op, models, gain]) => (
                    <tr key={op} className="border-b border-ink/15 last:border-b-0">
                      <td className="px-4 py-3 align-top font-mono text-[0.8rem]">{op}</td>
                      <td className="px-4 py-3 align-top text-ink/70">{models}</td>
                      <td className="px-4 py-3 align-top text-ink/70">{gain}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </section>

        <section id="cli" className="border-t border-ink/15">
          <div className="mx-auto max-w-3xl px-5 py-14">
            <p className="mono-label">CLI</p>
            <h2 className="display mt-3 text-3xl">REFERENCE</h2>
            <div className="mt-6 space-y-6">
              {cli.map(([cmd, desc]) => (
                <div key={cmd}>
                  <code className="font-mono text-[0.85rem] text-ink">{cmd}</code>
                  <p className="mt-1 text-sm leading-relaxed text-ink/70">{desc}</p>
                </div>
              ))}
            </div>
            <h3 className="display mt-10 text-xl">Flags</h3>
            <div className="mt-4 space-y-3">
              {flags.map(([flag, desc]) => (
                <div key={flag} className="flex flex-col gap-1 sm:flex-row sm:gap-4">
                  <code className="font-mono text-[0.8rem] text-ink sm:w-64 sm:shrink-0">{flag}</code>
                  <p className="text-sm leading-relaxed text-ink/70">{desc}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="report" className="border-t border-ink/15">
          <div className="mx-auto max-w-3xl px-5 py-14">
            <p className="mono-label">Report</p>
            <h2 className="display mt-3 text-3xl">WHAT A VERDICT MEANS</h2>
            <div className="mt-6 space-y-6">
              {[
                [
                  "Killed",
                  "At least one test failed under the mutant. The suite caught the injected bug class.",
                ],
                [
                  "Survived",
                  "Every test passed under the mutant. The injected bug class is invisible to the suite — the finding that matters.",
                ],
                [
                  "Build-failed",
                  "The mutant does not compile. Counted as a non-escape: the mutation provably broke the program.",
                ],
                [
                  "Timeout",
                  "The mutant exceeded the per-mutant time budget. Inconclusive, and excluded from the score — a run that learns nothing scores 0%, never a vacuous 100%.",
                ],
              ].map(([t, d]) => (
                <div key={t} className="flex flex-col gap-1 sm:flex-row sm:gap-4">
                  <span className="mono-label w-32 shrink-0 text-ink sm:pt-0.5">{t}</span>
                  <p className="text-sm leading-relaxed text-ink/70">{d}</p>
                </div>
              ))}
            </div>
            <p className="mt-8 text-sm leading-relaxed text-ink/70">
              A run produces JSON, a self-contained HTML scorecard, and a
              dashboard payload. The live panel renders the real demo run —{" "}
              <Link to="/dashboard" className="underline decoration-ink/40 underline-offset-4 hover:decoration-ink">
                mutanchor.vercel.app/dashboard
              </Link>
              .
            </p>
          </div>
        </section>
      </main>
      <Footer />
    </div>
  );
}