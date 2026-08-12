import { Link } from "react-router-dom";

const anchors = [
  { href: "/#problem", label: "Problem" },
  { href: "/#mechanism", label: "Mechanism" },
  { href: "/#operators", label: "Operators" },
  { href: "/#report", label: "Report" },
];

export function Nav() {
  return (
    <header className="sticky top-0 z-50 border-b border-ink/15 bg-bone/90 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-3.5">
        <Link to="/" className="display text-lg tracking-wide text-ink">
          MUTANCHOR
        </Link>
        <nav className="hidden items-center gap-6 md:flex">
          {anchors.map((a) => (
            <a
              key={a.href}
              href={a.href}
              className="text-[0.72rem] uppercase tracking-[0.14em] text-ink/60 transition-colors hover:text-ink"
            >
              {a.label}
            </a>
          ))}
          <a
            href="https://github.com/subheeksh5599/mutanchor"
            className="text-[0.72rem] uppercase tracking-[0.14em] text-ink/60 transition-colors hover:text-ink"
          >
            GitHub
          </a>
        </nav>
        <Link
          to="/dashboard"
          className="border-2 border-ink bg-chartreuse px-3.5 py-1.5 text-[0.72rem] font-medium uppercase tracking-[0.14em] text-ink transition-transform hover:-translate-y-0.5"
        >
          Open report
        </Link>
      </div>
    </header>
  );
}
