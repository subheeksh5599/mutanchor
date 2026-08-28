const anchors = [
  { href: "#quickstart", label: "Quickstart" },
  { href: "#operators", label: "Operators" },
  { href: "#cli", label: "CLI" },
  { href: "#report", label: "Report" },
];

export function Nav() {
  return (
    <header className="sticky top-0 z-50 border-b border-ink/15 bg-bone/90 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-3.5">
        <a href="#what" className="display text-lg tracking-wide text-ink">
          MUTANCHOR
        </a>
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
        <a
          href="https://github.com/subheeksh5599/mutanchor"
          className="border-2 border-ink bg-accent px-3.5 py-1.5 text-[0.72rem] font-medium uppercase tracking-[0.14em] text-ink transition-transform hover:-translate-y-0.5"
        >
          View source
        </a>
      </div>
    </header>
  );
}
