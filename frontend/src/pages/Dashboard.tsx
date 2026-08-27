import { useEffect, useState } from "react";
import { Nav } from "../components/Nav";
import { Footer } from "../components/Footer";
import { loadReport, type MutationReport } from "../lib/report";

function pct(score: number) {
  return `${Math.round(score * 100)}%`;
}

function StatCell({
  label,
  value,
  tone,
}: {
  label: string;
  value: number | string;
  tone?: "accent" | "danger" | "plain";
}) {
  const dot =
    tone === "accent"
      ? "bg-accent"
      : tone === "danger"
        ? "bg-danger"
        : "bg-ink/25";
  return (
    <div className="border-2 border-ink bg-bone p-4">
      <div className="flex items-center gap-2">
        <span className={`inline-block h-2 w-2 ${dot}`} />
        <span className="mono-label">{label}</span>
      </div>
      <div className="display mt-2 text-3xl text-ink">{value}</div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="mx-auto max-w-6xl px-5 py-24">
      <div className="border-2 border-ink bg-bone p-8 sm:p-10">
        <p className="display text-3xl text-ink">No report yet</p>
        <p className="mt-4 max-w-xl text-sm leading-relaxed text-ink/70">
          This panel renders the real output of mutanchor run. Run the CLI on
          your Anchor program and place the generated report.json in the site
          root. Until that file exists, nothing is shown and nothing is
          fabricated.
        </p>
        <a
          href="https://github.com/subheeksh5599/mutanchor"
          className="mt-6 inline-block border-2 border-ink px-5 py-3 text-[0.75rem] font-medium uppercase tracking-[0.14em] text-ink transition-colors hover:bg-ink hover:text-bone"
        >
          Source on GitHub
        </a>
      </div>
    </div>
  );
}

function ReportView({ report }: { report: MutationReport }) {
  return (
    <div className="mx-auto max-w-6xl px-5 py-16">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="mono-label">Report</p>
          <h1 className="display mt-2 text-4xl text-ink sm:text-5xl">
            {report.program}
          </h1>
        </div>
        <p className="font-mono text-[0.7rem] uppercase tracking-[0.16em] text-ink/50">
          generated{" "}
          {new Date(report.generatedAt).toLocaleString("en-GB", {
            timeZone: "UTC",
            year: "numeric",
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
          })}{" "}
          UTC
        </p>
      </div>

      <div className="mt-10 grid grid-cols-2 gap-4 md:grid-cols-5">
        <div className="border-2 border-ink bg-accent p-4 shadow-[6px_6px_0_0_var(--color-ink)]">
          <span className="mono-label">Mutation score</span>
          <div className="display mt-2 text-3xl text-ink">
            {pct(report.mutationScore)}
          </div>
        </div>
        <StatCell label="Killed" value={report.killed} tone="accent" />
        <StatCell label="Survived" value={report.survived} tone="danger" />
        <StatCell label="Build failed" value={report.buildFailed} />
        <StatCell label="Timeout" value={report.timeout} />
      </div>

      <h2 className="display mt-16 text-2xl text-ink">PER INSTRUCTION</h2>
      <div className="mt-4 border-2 border-ink">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b-2 border-ink">
              <th className="mono-label px-4 py-3">Instruction</th>
              <th className="mono-label px-4 py-3">Killed</th>
              <th className="mono-label px-4 py-3">Survived</th>
              <th className="mono-label px-4 py-3">Total</th>
              <th className="mono-label hidden px-4 py-3 sm:table-cell">
                Score
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-ink/15">
            {report.instructions.map((ins) => (
              <tr key={ins.name} className="hover:bg-bone/60">
                <td className="px-4 py-3.5 font-mono text-ink">{ins.name}</td>
                <td className="px-4 py-3.5 text-ink/75">{ins.killed}</td>
                <td className="px-4 py-3.5">
                  <span className={ins.survived > 0 ? "text-danger" : "text-ink/75"}>
                    {ins.survived}
                  </span>
                </td>
                <td className="px-4 py-3.5 text-ink/75">{ins.total}</td>
                <td className="hidden px-4 py-3.5 sm:table-cell">
                  <div className="flex items-center gap-3">
                    <div className="h-2.5 w-28 border border-ink/30 bg-bone">
                      <div
                        className={
                          ins.survived > 0
                            ? "h-full bg-danger/80"
                            : "h-full bg-accent"
                        }
                        style={{ width: pct(ins.score) }}
                      />
                    </div>
                    <span className="font-mono text-xs text-ink/70">
                      {pct(ins.score)}
                    </span>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="display mt-16 text-2xl text-ink">SURVIVING MUTANTS</h2>
      {report.survivors.length === 0 ? (
        <p className="mt-4 border-l-2 border-accent bg-bone px-5 py-4 text-sm text-ink/75">
          No survivors. Every mutant was killed by the test suite.
        </p>
      ) : (
        <div className="mt-4 space-y-4">
          {report.survivors.map((m) => (
            <div key={m.id} className="border-2 border-ink bg-bone p-5">
              <div className="flex flex-wrap items-baseline justify-between gap-3">
                <span className="font-mono text-sm text-danger">{m.operator}</span>
                <span className="font-mono text-xs text-ink/50">
                  {m.file}:{m.line}
                </span>
              </div>
              <p className="mt-3 text-sm text-ink/75">{m.reason}</p>
              <p className="mt-3 border-l-2 border-danger pl-4 text-sm leading-relaxed text-ink/80">
                <span className="mono-label">Attacker</span>{" "}
                <span className="mt-1 block">{m.exploit}</span>
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function Dashboard() {
  const [state, setState] = useState<"loading" | "empty" | "loaded">("loading");
  const [report, setReport] = useState<MutationReport | null>(null);

  useEffect(() => {
    let live = true;
    loadReport().then((r) => {
      if (!live) return;
      setReport(r);
      setState(r ? "loaded" : "empty");
    });
    return () => {
      live = false;
    };
  }, []);

  return (
    <div className="min-h-screen bg-bone">
      <Nav />
      <main>
        {state === "loading" && (
          <div className="mx-auto max-w-6xl px-5 py-24">
            <p className="mono-label">Checking for a report</p>
          </div>
        )}
        {state === "empty" && <EmptyState />}
        {state === "loaded" && report && <ReportView report={report} />}
      </main>
      <Footer />
    </div>
  );
}
