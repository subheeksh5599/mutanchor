import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { Nav } from "../components/Nav";
import { Footer } from "../components/Footer";
import { KillGrid } from "../components/KillGrid";

gsap.registerPlugin(useGSAP, ScrollTrigger);

const operators: Array<[string, string]> = [
  ["signer check removal", "missing signer validation"],
  ["PDA seed swap", "wrong seeds, wrong account resolution"],
  ["bump mismatch", "using the wrong PDA bump"],
  ["authority check drop", "missing owner or authority validation"],
  ["discriminator check removal", "accepting wrong instruction discriminators"],
  ["CPI target swap", "calling the wrong program"],
  ["close or rent check drop", "accounts closed or not rent exempt, unvalidated"],
  ["comparison flip", "boundary errors, off by one, wrong operator"],
];

const steps = [
  {
    title: "Locate",
    body: "init parses the Anchor IDL and maps every instruction to its source file.",
  },
  {
    title: "Mutate",
    body: "syn and quote rewrite the Rust AST. One fault per mutant, no compound mutations.",
  },
  {
    title: "Execute",
    body: "cargo build-sbf compiles each mutant. Your test suite runs against it on LiteSVM, in process, no validator.",
  },
  {
    title: "Score",
    body: "Every mutant resolves to killed, survived, build-failed or timeout. The report scores each instruction.",
  },
];

function Mechanism() {
  const [active, setActive] = useState(-1);
  const refs = useRef<Array<HTMLDivElement | null>>([]);

  useEffect(() => {
    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) {
            const i = Number((e.target as HTMLElement).dataset.step);
            setActive((prev) => Math.max(prev, i));
          }
        }
      },
      { threshold: 0.45 },
    );
    refs.current.forEach((el) => el && obs.observe(el));
    return () => obs.disconnect();
  }, []);

  const pct = Math.round(((active + 1) / steps.length) * 100);

  return (
    <section id="mechanism" className="mx-auto max-w-6xl px-5 py-24">
      <p className="mono-label">Mechanism</p>
      <h2 className="display reveal mt-3 max-w-3xl text-4xl sm:text-5xl">
        MUTATE, BUILD, EXECUTE, <span className="serif-it">SCORE</span>
      </h2>

      <div className="relative mt-14 border-l-2 border-ink/20 pl-8 sm:pl-12">
        <div
          className="absolute left-[-2px] top-0 w-[3px] bg-ink transition-all duration-700 ease-out"
          style={{ height: `${pct}%` }}
        />
        {steps.map((s, i) => (
          <div
            key={s.title}
            ref={(el) => {
              refs.current[i] = el;
            }}
            data-step={i}
            className={`step-cell mb-8 border-2 border-ink/20 bg-bone p-5 sm:p-6 ${
              i <= active ? "is-active" : ""
            }`}
          >
            <div className="display text-2xl text-ink sm:text-3xl">{s.title}</div>
            <p className="mt-2 max-w-xl text-sm leading-relaxed text-ink/70">
              {s.body}
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}

export default function Landing() {
  const rootRef = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      if (reduced) return;
      gsap.from(".hero-reveal", {
        y: 26,
        opacity: 0,
        duration: 0.7,
        stagger: 0.09,
        ease: "power2.out",
      });
      gsap.utils.toArray<HTMLElement>(".reveal").forEach((el) => {
        gsap.from(el, {
          scrollTrigger: { trigger: el, start: "top 85%" },
          y: 24,
          opacity: 0,
          duration: 0.6,
          ease: "power2.out",
        });
      });
    },
    { scope: rootRef },
  );

  return (
    <div ref={rootRef} className="bg-bone">
      <Nav />

      <main>
        <section className="mx-auto grid max-w-6xl gap-12 px-5 pb-20 pt-16 sm:pt-24 md:grid-cols-2 md:items-center">
          <div>
            <p className="mono-label hero-reveal">Solana Anchor</p>
            <h1 className="display hero-reveal mt-4 text-[clamp(2.6rem,6vw,4.6rem)]">
              Mutation testing for{" "}
              <span className="serif-it">Solana</span> Anchor programs
            </h1>
            <p className="hero-reveal mt-6 max-w-xl text-sm leading-relaxed text-ink/70">
              Mutanchor rewrites Anchor source at the AST level to inject real
              bug classes, builds each mutant, runs your test suite against it
              on LiteSVM, and reports which mutants survive. A survivor is a bug
              your tests would miss in production. It is a plain CLI, no skill
              and no agent: every change comes from a fixed rule set, not AI.
            </p>
            <div className="hero-reveal mt-8 flex flex-wrap gap-4">
              <Link
                to="/dashboard"
                className="border-2 border-ink bg-chartreuse px-5 py-3 text-[0.75rem] font-medium uppercase tracking-[0.14em] text-ink transition-transform hover:-translate-y-0.5"
              >
                Open the report
              </Link>
              <a
                href="https://github.com/subheeksh5599/mutanchor"
                className="border-2 border-ink px-5 py-3 text-[0.75rem] font-medium uppercase tracking-[0.14em] text-ink transition-colors hover:bg-ink hover:text-bone"
              >
                Source on GitHub
              </a>
            </div>
          </div>
          <div className="hero-reveal">
            <KillGrid />
          </div>
        </section>

        <section id="problem" className="border-t border-ink/15">
          <div className="mx-auto max-w-6xl px-5 py-24">
            <p className="mono-label reveal">Problem</p>
            <h2 className="display reveal mt-3 text-4xl sm:text-5xl">
              LINE COVERAGE <span className="serif-it">LIES</span>
            </h2>

            <div className="mt-12 grid gap-10 lg:grid-cols-2">
              <div className="space-y-8">
                {[
                  [
                    "Coverage counts execution, not assertions.",
                    "A suite can touch every line and still miss every bug that matters. The percentage says nothing about whether the checks are real.",
                  ],
                  [
                    "Anchor bugs are repetitive.",
                    "Solana audits keep finding the same classes: missing signer checks, wrong PDA bumps, dropped authority checks. If your tests do not catch one, an attacker can use it.",
                  ],
                  [
                    "Nothing measures the gap.",
                    "cargo-mutants targets generic Rust with zero Solana support. Test generators produce tests. Nothing verifies the tests would fail when the program is broken.",
                  ],
                ].map(([head, body]) => (
                  <div key={head} className="border-l-2 border-ink/25 pl-5">
                    <h3 className="display text-xl text-ink">{head}</h3>
                    <p className="mt-2 text-sm leading-relaxed text-ink/70">
                      {body}
                    </p>
                  </div>
                ))}
              </div>

              <div className="reveal h-fit border-2 border-ink">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr className="border-b-2 border-ink">
                      <th className="mono-label px-4 py-3"> </th>
                      <th className="mono-label px-4 py-3">Line coverage</th>
                      <th className="mono-label px-4 py-3">Mutation score</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-ink/15">
                    {[
                      ["Measures", "lines executed", "faults detected"],
                      ["Misses", "weak assertions", "nothing, by construction"],
                      [
                        "100% means",
                        "every line ran",
                        "every injected fault was caught",
                      ],
                      [
                        "Gamed by",
                        "tests that assert nothing",
                        "only real assertions count",
                      ],
                    ].map(([k, a, b]) => (
                      <tr key={k}>
                        <td className="mono-label px-4 py-3.5 align-top">{k}</td>
                        <td className="px-4 py-3.5 text-ink/75">{a}</td>
                        <td className="px-4 py-3.5 text-ink">{b}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </section>

        <Mechanism />

        <section id="operators" className="border-t border-bone/10 bg-pitch text-bone">
          <div className="mx-auto max-w-6xl px-5 py-24">
            <p className="font-mono text-[0.7rem] uppercase tracking-[0.18em] text-chartreuse">
              Operators
            </p>
            <h2 className="display reveal mt-3 text-4xl sm:text-5xl">
              THE BUG CLASSES AUDITS KEEP <span className="serif-it">FINDING</span>
            </h2>
            <p className="mt-5 max-w-2xl text-sm leading-relaxed text-bone/65">
              Each operator models a fault from the Solana audit corpus. Kill
              the mutant and your tests proved something. Let it survive and
              the report tells you where, and what an attacker could do with
              it.
            </p>

            <div className="mt-14 border-t border-bone/15">
              {operators.map(([op, cls]) => (
                <div
                  key={op}
                  className="reveal grid gap-1 border-b border-bone/15 py-4 transition-colors hover:bg-bone/5 sm:grid-cols-2 sm:gap-6 sm:px-3"
                >
                  <div className="font-mono text-sm text-chartreuse">
                    {op}
                  </div>
                  <div className="text-sm text-bone/70">{cls}</div>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="report" className="border-t border-ink/15">
          <div className="mx-auto max-w-6xl px-5 py-24">
            <p className="mono-label reveal">Report</p>
            <h2 className="display reveal mt-3 text-4xl sm:text-5xl">
              WHAT THE REPORT <span className="serif-it">SHOWS</span>
            </h2>

            <div className="mt-12 grid gap-10 lg:grid-cols-2">
              <ul className="space-y-5 text-sm leading-relaxed text-ink/75">
                <li className="border-l-2 border-ink/25 pl-5">
                  Per-instruction mutation score, plus the overall score.
                </li>
                <li className="border-l-2 border-ink/25 pl-5">
                  Surviving mutants with operator, file and line, and the
                  reason they survived.
                </li>
                <li className="border-l-2 border-ink/25 pl-5">
                  Exploit annotation per survivor: what an attacker could do
                  with it.
                </li>
                <li className="border-l-2 border-ink/25 pl-5">
                  Static HTML for humans, JSON for CI.
                </li>
              </ul>

              <div className="reveal h-fit border-2 border-ink bg-bone p-6">
                <p className="mono-label">Data honesty</p>
                <p className="mt-3 text-sm leading-relaxed text-ink/75">
                  The report panel renders the real output of mutanchor run. It
                  stays empty until a run exists. Nothing on this site is
                  sample data.
                </p>
                <Link
                  to="/dashboard"
                  className="mt-6 inline-block border-2 border-ink bg-chartreuse px-5 py-3 text-[0.75rem] font-medium uppercase tracking-[0.14em] text-ink transition-transform hover:-translate-y-0.5"
                >
                  Open the report panel
                </Link>
              </div>
            </div>
          </div>
        </section>

        <section className="border-t-2 border-ink bg-chartreuse">
          <div className="mx-auto flex max-w-6xl flex-col items-start justify-between gap-8 px-5 py-16 md:flex-row md:items-center">
            <h2 className="display text-4xl text-ink sm:text-5xl">
              MEASURE WHAT YOUR TESTS PROVE
            </h2>
            <div className="flex flex-wrap gap-4">
              <Link
                to="/dashboard"
                className="border-2 border-ink bg-ink px-5 py-3 text-[0.75rem] font-medium uppercase tracking-[0.14em] text-bone transition-transform hover:-translate-y-0.5"
              >
                Open the report
              </Link>
              <a
                href="https://github.com/subheeksh5599/mutanchor"
                className="border-2 border-ink px-5 py-3 text-[0.75rem] font-medium uppercase tracking-[0.14em] text-ink transition-colors hover:bg-ink hover:text-bone"
              >
                Source on GitHub
              </a>
            </div>
          </div>
        </section>
      </main>

      <Footer />
    </div>
  );
}
