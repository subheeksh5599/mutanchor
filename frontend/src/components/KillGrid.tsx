import { useMemo } from "react";

function hashStr(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function mulberry32(seed: number) {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

type TileKind = "killed" | "survived" | "neutral";

export function KillGrid() {
  const tiles = useMemo<TileKind[]>(() => {
    const rng = mulberry32(hashStr("mutanchor"));
    return Array.from({ length: 90 }, () => {
      const r = rng();
      if (r < 0.56) return "killed";
      if (r < 0.74) return "survived";
      return "neutral";
    });
  }, []);

  return (
    <div>
      <div className="relative overflow-hidden border-2 border-ink bg-bone">
        <div className="grid grid-cols-10 gap-1 p-1.5 sm:grid-cols-15">
          {tiles.map((kind, i) => (
            <div
              key={i}
              className={
                kind === "killed"
                  ? "aspect-square bg-chartreuse"
                  : kind === "survived"
                    ? "tile-survivor aspect-square bg-danger"
                    : "aspect-square border border-ink/25"
              }
            />
          ))}
        </div>
        <div className="scan-line pointer-events-none absolute inset-x-0 top-0 h-2 bg-chartreuse/50" />
      </div>
      <div className="mt-3 flex items-center gap-5 text-[0.65rem] uppercase tracking-[0.16em] text-ink/60">
        <span className="flex items-center gap-1.5">
          <span className="inline-block h-2.5 w-2.5 bg-chartreuse" />
          killed by tests
        </span>
        <span className="flex items-center gap-1.5">
          <span className="inline-block h-2.5 w-2.5 bg-danger" />
          survived
        </span>
        <span className="flex items-center gap-1.5">
          <span className="inline-block h-2.5 w-2.5 border border-ink/40" />
          untested
        </span>
      </div>
    </div>
  );
}
