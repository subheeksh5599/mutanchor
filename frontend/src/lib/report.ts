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
  try {
    const res = await fetch("report.json", { cache: "no-store" });
    if (!res.ok) return null;
    return (await res.json()) as MutationReport;
  } catch {
    return null;
  }
}
