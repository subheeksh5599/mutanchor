export interface InstructionScore {
  name: string;
  killed: number;
  survived: number;
  total: number;
  score: number;
}

export interface SurvivingMutant {
  id: string;
  operator: string;
  file: string;
  line: number;
  reason: string;
  exploit: string;
}

export interface MutationReport {
  program: string;
  generatedAt: string;
  totalMutants: number;
  killed: number;
  survived: number;
  buildFailed: number;
  timeout: number;
  mutationScore: number;
  instructions: InstructionScore[];
  survivors: SurvivingMutant[];
}

export async function loadReport(): Promise<MutationReport | null> {
  return loadReportFromPath("/report.json");
}

export async function loadReportFromPath(
  path: string,
): Promise<MutationReport | null> {
  try {
    const res = await fetch(path, { cache: "no-store" });
    if (!res.ok) return null;
    return (await res.json()) as MutationReport;
  } catch {
    return null;
  }
}

export const KNOWN_REPORTS: Array<{ path: string; label: string; note: string }> = [
  {
    path: "/report.json",
    label: "demo-vault",
    note: "In-repo Anchor vault (5 instructions, 10-test LiteSVM suite)",
  },
  {
    path: "/report-registry.json",
    label: "demo-registry",
    note: "In-repo Anchor name-registry (4 instructions, 10-test LiteSVM suite) — second data point, different bug surface",
  },
];
