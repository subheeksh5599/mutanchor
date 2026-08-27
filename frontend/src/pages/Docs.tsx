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
  ["unchecked math", "silent arithmetic overflow (checked_* dropped)", "balances wrap; overflow-in-price / mint-inflation escapes tests"],
  ["realloc check drop", "unchecked account resize / re-init", "account resized past its rent-exempt / expected layout"],
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

const toc: Array<[string, string]> = [
  ["what", "Overview"],
  ["why", "Why mutation testing"],
  ["install", "Install"],
  ["quickstart", "Quickstart"],
  ["operators", "Operators"],
  ["cli", "CLI"],
  ["report", "Report"],
];

function CodeBlock({ label, children }: { label: string; children: string }) {
  return (
    <div className="mt-6 overflow-hidden border-2 border-ink">
      <div className="flex items-center justify-between border-b-2 border-ink bg-ink px-4 py-2 text-[0.65rem] uppercase tracking-[0.18em] text-bone/70">
        <span>{label}</span>
        <span className="text-bone/40">shell</span>
      </div>
      <pre className="overflow-x-auto bg-pitch p-5 text-[0.8rem] leading-relaxed text-bone">
        <code>{children}</code>
      </pre>
    </div>
  );
}

export default function Docs() {
  return (
    <div className="bg-bone">
      <Nav />
      <main>
        <div className="mx-auto grid max-w-6xl grid-cols-1 gap-10 px-5 pb-16 pt-12 lg:grid-cols-[220px_minmax(0,1fr)]">
          {/* sidebar TOC */}
          <aside className="hidden lg:block">
            <div className="sticky top-24">
              <p className="mono-label">Contents</p>
              <nav className="mt-4 flex flex-col gap-2 border-l border-ink/20 pl-4">
                {toc.map(([id, label]) => (
                  <a
                    key={id}
                    href={`#${id}`}
                    className="text-[0.78rem] text-ink/60 transition-colors hover:text-ink"
                  >
                    {label}
                  </a>
                ))}
              </nav>
              <div className="mt-8 border-l border-ink/20 pl-4">
                <p className="mono-label">External</p>
                <a
                  href="https://github.com/subheeksh5599/mutanchor"
                  className="mt-2 block text-[0.78rem] text-ink/60 transition-colors hover:text-ink"
                >
                  GitHub source →
                </a>
                <Link
                  to="/dashboard"
                  className="mt-1 block text-[0.78rem] text-ink/60 transition-colors hover:text-ink"
                >
                  Live report →
                </Link>
              </div>
            </div>
          </aside>

          {/* content column */}
          <div className="min-w-0">
            <section id="what" className="pb-4 pt-2">
              <p className="mono-label">Solana Anchor</p>
              <h1 className="display mt-3 text-[clamp(2.2rem,5vw,3.6rem)] leading-[1.05]">
                Mutation testing for Solana Anchor programs
              </h1>
              <p className="mt-6 max-w-2xl text-sm leading-relaxed text-ink/70">
                Mutanchor rewrites Anchor program source at the AST level to inject
                real bug classes, builds each mutant with{" "}
                <code className="font-mono text-[0.8em]">cargo build-sbf</code>,
                runs the program's own test suite against it headlessly on LiteSVM,
                and reports which mutants survive. A survivor is a bug your tests
                would miss in production.
              </p>
              <p className="mt-4 max-w-2xl text-sm leading-relaxed text-ink/70">
                It is a local CLI. Every change comes from a fixed rule set, not an
                AI: same program, same mutants, deterministic output. No cluster is
                involved — execution is fully in-process.
              </p>
            </section>

            <section id="why" className="mt-12 border-t border-ink/15 pt-12">
              <p className="mono-label">Rationale</p>
              <h2 className="display mt-3 text-3xl">WHY MUTATION TESTING</h2>
              <div className="mt-5 grid gap-5 text-sm leading-relaxed text-ink/75 sm:grid-cols-2">
                <div className="border-l-2 border-ink/60 pl-4">
                  <p className="font-mono text-[0.72rem] uppercase tracking-[0.14em] text-ink">
                    Coverage measures the wrong thing
                  </p>
                  <p className="mt-2">
                    Line coverage records which lines executed. It does not
                    record whether assertions actually check anything meaningful.
                    You can hit 100% coverage with zero assertions.
                  </p>
                </div>
                <div className="border-l-2 border-ink/60 pl-4">
                  <p className="font-mono text-[0.72rem] uppercase tracking-[0.14em] text-ink">
                    Anchor bugs are repetitive
                  </p>
                  <p className="mt-2">
                    Real audit reports keep flagging the same eight patterns.
                    Mutanchor's operators map 1:1 to those patterns, so a
                    surviving mutant maps directly to a real audit finding shape.
                  </p>
                </div>
                <div className="border-l-2 border-ink/60 pl-4">
                  <p className="font-mono text-[0.72rem] uppercase tracking-[0.14em] text-ink">
                    Deterministic, not AI
                  </p>
                  <p className="mt-2">
                    The analysis path has no model. Same input, same output.
                    Every verdict is a real{" "}
                    <code className="font-mono text-[0.8em]">cargo build-sbf</code>{" "}
                    + LiteSVM run — replayable, reviewable, citable in an audit
                    report.
                  </p>
                </div>
                <div className="border-l-2 border-ink/60 pl-4">
                  <p className="font-mono text-[0.72rem] uppercase tracking-[0.14em] text-ink">
                    Honest scoring
                  </p>
                  <p className="mt-2">
                    Timeouts are excluded from the score. A run that learns
                    nothing scores 0%, never a vacuous 100%. Build failures count
                    as non-escapes; the mutation provably broke the program.
                  </p>
                </div>
              </div>
            </section>

            <section id="install" className="mt-16 border-t border-ink/15 pt-12">
              <p className="mono-label">Install</p>
              <h2 className="display mt-3 text-3xl">BUILD THE CLI</h2>
              <CodeBlock label="terminal">{`git clone https://github.com/subheeksh5599/mutanchor.git
cd mutanchor
cargo build --release
# binary at target/release/mutanchor`}</CodeBlock>
              <p className="mt-4 max-w-2xl text-sm leading-relaxed text-ink/70">
                Requires a Rust toolchain (2021 edition or newer) and the Solana
                build toolchain (<code className="font-mono text-[0.8em]">cargo build-sbf</code>).
                The demo site is only a viewer for published reports; the tool
                itself runs locally.
              </p>
            </section>

            <section id="quickstart" className="mt-16 border-t border-ink/15 pt-12">
              <p className="mono-label">Quickstart</p>
              <h2 className="display mt-3 text-3xl">INIT — RUN — REPORT</h2>

              <CodeBlock label="mutanchor init">{`$ mutanchor init demo/demo-vault
program dir: demo/demo-vault
instructions (5):
   24  create                   src/lib.rs
   30  deposit                  src/lib.rs
   42  withdraw                 src/lib.rs
   54  pay                      src/lib.rs
   72  close                    src/lib.rs`}</CodeBlock>

              <CodeBlock label="mutanchor run">{`$ mutanchor run demo/demo-vault
generated 18 mutants (0 equivalent/duplicate dropped) across 1 file(s)
priming incremental build cache (pristine build + test)…
cache primed
[1/18] authority_check_drop at src/lib.rs:32
[2/18] authority_check_drop at src/lib.rs:45
...
mutation score: 72.2% (9 killed, 5 survived, 4 build-failed, 0 timeout)
report.json:  target/mutanchor/report.json
report.html:  target/mutanchor/report.html
dashboard.json: target/mutanchor/dashboard.json`}</CodeBlock>

              <CodeBlock label="mutanchor report / ci">{`$ mutanchor report target/mutanchor/report.json --html report.html
$ mutanchor ci demo/demo-vault --max-survivors 0
surviving mutants: 5 (max allowed: 0)
error: CI gate: 5 surviving mutants exceed max of 0`}</CodeBlock>

              <p className="mt-6 max-w-2xl text-sm leading-relaxed text-ink/70">
                The in-repo demo program (<code className="font-mono text-[0.8em]">demo/demo-vault</code>,
                5 instructions, 10-test LiteSVM suite) is the reference run:{" "}
                <Link to="/dashboard" className="underline decoration-ink/40 underline-offset-4 hover:decoration-ink">
                  the live report
                </Link>
                .
              </p>
            </section>

            <section id="operators" className="mt-16 border-t border-ink/15 pt-12">
              <p className="mono-label">Operators</p>
              <h2 className="display mt-3 text-3xl">TEN BUG CLASSES</h2>
              <p className="mt-4 max-w-2xl text-sm leading-relaxed text-ink/70">
                Each operator models a recurring class from real Solana audit
                findings. One operator fires per location; a mutant is exactly
                one fault.
              </p>
              <div className="mt-6 overflow-x-auto border-2 border-ink">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b-2 border-ink bg-paper">
                      <th className="mono-label px-4 py-3">Operator</th>
                      <th className="mono-label px-4 py-3">Models</th>
                      <th className="mono-label px-4 py-3">What an attacker gains</th>
                    </tr>
                  </thead>
                  <tbody>
                    {operators.map(([op, models, gain]) => (
                      <tr key={op} className="border-b border-ink/15 last:border-b-0 hover:bg-paper/60">
                        <td className="px-4 py-3 align-top font-mono text-[0.8rem]">{op}</td>
                        <td className="px-4 py-3 align-top text-ink/70">{models}</td>
                        <td className="px-4 py-3 align-top text-ink/70">{gain}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>

            <section id="cli" className="mt-16 border-t border-ink/15 pt-12">
              <p className="mono-label">CLI</p>
              <h2 className="display mt-3 text-3xl">REFERENCE</h2>
              <div className="mt-6 space-y-6">
                {cli.map(([cmd, desc]) => (
                  <div key={cmd} className="border-l-2 border-ink/60 pl-4">
                    <code className="font-mono text-[0.85rem] text-ink">{cmd}</code>
                    <p className="mt-1 max-w-2xl text-sm leading-relaxed text-ink/70">{desc}</p>
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
            </section>

            <section id="report" className="mt-16 border-t border-ink/15 pt-12">
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
                    <p className="max-w-2xl text-sm leading-relaxed text-ink/70">{d}</p>
                  </div>
                ))}
              </div>
              <p className="mt-8 max-w-2xl text-sm leading-relaxed text-ink/70">
                A run produces JSON, a self-contained HTML scorecard, and a
                dashboard payload. The live panel renders the real demo run —{" "}
                <Link to="/dashboard" className="underline decoration-ink/40 underline-offset-4 hover:decoration-ink">
                  mutanchor.vercel.app/dashboard
                </Link>
                .
              </p>
            </section>
          </div>
        </div>
      </main>
      <Footer />
    </div>
  );
}
