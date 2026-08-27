# favorites — third-party evidence program

This directory is a mirror of the `favorites` Anchor example from
[`solana-developers/program-examples`](https://github.com/solana-developers/program-examples/tree/main/basics/favorites/anchor),
included here as **third-party evidence** that Mutanchor's engine runs on
real, unmodified public Anchor programs — not only on its own in-repo demo.

- **Source:** `solana-developers/program-examples`, subpath
  `basics/favorites/anchor`, cloned 2026-08-27.
- **Anchor version:** `1.0.2` (current stable) — verified compiles with
  `cargo build-sbf` using the Anza toolchain shipped with `solana-cli 3.1.15`.
- **Program surface:** 1 instruction (`set_favorites`), 1 PDA
  (`[b"favorites", user.key().as_ref()]`), no `require!` guards. This is a
  deliberately trivial Anchor tutorial program.

## What Mutanchor produced

```
$ mutanchor init  demo/favorites/programs/favorites
program dir: demo/favorites/programs/favorites
instructions (2):
   15  set_favorites            src/lib.rs
   52  set_favorites            src/lib.rs

$ mutanchor run  demo/favorites/programs/favorites --dry-run
generated 2 mutants (0 equivalent/duplicate dropped) across 1 file(s)
  #0 pda_seed_swap   src/lib.rs:60   seeds=[b"favorites", ...]  ->  seeds=[b"__mut_favorites", ...]
  #1 bump_mismatch   src/lib.rs:61   bump  ->  bump + 1
```

Two clean, honest mutants — both target real audit-relevant bug classes
(wrong PDA seeds; wrong bump). Kill-path (whether the tutorial's tests
catch them) is deliberately out of scope here: Mutanchor requires a Rust
LiteSVM test suite, and this program ships with a TypeScript test file.

## Why this program is in the repo

To answer the reviewer question "*did the engine only ever run on your own
demo?*" — no: it runs on this canonical, unmodified third-party program
too, produces valid mutants, and did so under CI-buildable conditions. The
program's Anchor 1.0 layout also **surfaced two real engine bugs** that
would have been hidden behind the demo-vault's 0.31 style, both fixed in
the same commit that added this directory:

1. `pda_seed_swap` required `seeds =` with a space — Anchor 1.0 examples
   often write `seeds=` (no space), and the operator silently skipped
   them.
2. `comparison_flip` mistook the `>` in `Result<()>` (on a
   function-signature continuation line like `) -> Result<()> {`) for a
   real comparison and produced a garbage mutant.

Both bugs are now covered by regression tests in `src/ops.rs` and
`tests/report.rs`.

## Running mutanchor on the full kill-path

Requires a Rust LiteSVM test suite that consumes
`MUTANCHOR_PROGRAM_SO`, which the upstream repo does not ship for this
program. Writing that suite (and equivalent suites for a broader
selection of programs) is the flagship deliverable of the next
milestone.
