# Mutanchor

Mutation testing for Solana Anchor programs.

## What

Mutanchor deliberately breaks Anchor programs and runs their test suite against
each broken version. A mutant that survives means the tests would miss that bug
class in production.

## Why

Anchor projects have no way to measure whether their tests actually catch the
bug classes Solana audits keep finding: missing signer checks, PDA bump
mismatches, dropped authority checks, discriminator removal, boundary
comparison flips, CPI target swaps.

cargo-mutants has zero Solana support. No Solana mutation tool ships. Mutanchor
fills that gap.

## Status

Early scaffold. CLI skeleton only (subcommands wired, no operators implemented
yet). Builds and runs.

## Architecture

```
mutate  ->  build (cargo build-sbf)  ->  execute (LiteSVM / anchor-litesvm)  ->  score
```

## Quickstart

```
cargo build --release
cargo run -- --help
```

## CLI

- `mutanchor init` — parse IDL, locate instruction source files
- `mutanchor run` — mutate, build each mutant, run the suite on LiteSVM
- `mutanchor report` — emit HTML + JSON report
- `mutanchor ci` — fail on surviving mutants above a threshold

## Operators (planned)

| Operator | Bug class it models |
|---|---|
| signer check removal | missing signer validation |
| PDA seed swap | wrong seeds, wrong account resolution |
| bump mismatch | using the wrong PDA bump |
| authority check drop | missing owner/authority validation |
| discriminator check removal | accepting wrong instruction discriminators |
| CPI target swap | calling the wrong program |
| close/rent check drop | accounts closed or rent-exempt not validated |
| comparison flip | boundary errors (off-by-one, wrong operator) |

## Report

Static HTML (deployable to Vercel) plus JSON for CI. Per-instruction mutation
score, surviving-mutant list with operator + file:line + exploit annotation.

## Demo site

Landing page and live report viewer at one URL. Frontend sourced separately,
static build deployed to Vercel. Link added when live.

## License

MIT
