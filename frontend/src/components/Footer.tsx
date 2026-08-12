export function Footer() {
  return (
    <footer className="border-t border-ink/15 bg-bone">
      <div className="mx-auto flex max-w-6xl flex-col items-start justify-between gap-4 px-5 py-8 sm:flex-row sm:items-center">
        <span className="display text-base text-ink">MUTANCHOR</span>
        <div className="flex items-center gap-5 text-[0.7rem] uppercase tracking-[0.14em] text-ink/55">
          <a
            href="https://github.com/subheeksh5599/mutanchor"
            className="transition-colors hover:text-ink"
          >
            Source
          </a>
          <a
            href="https://github.com/subheeksh5599/mutanchor/blob/main/LICENSE"
            className="transition-colors hover:text-ink"
          >
            MIT
          </a>
        </div>
      </div>
    </footer>
  );
}
