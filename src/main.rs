//! Mutanchor — CLI entry point.
//!
//! `init` maps instructions to source files; `run` mutates, builds and
//! tests each mutant on LiteSVM; `report` renders the scorecard; `ci` fails
//! the build when surviving mutants exceed a threshold.

use mutanchor::{engine, init, report};

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mutanchor",
    version,
    about = "Mutation testing for Solana Anchor programs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse the Anchor IDL and locate instruction source files
    Init {
        /// Path to the Anchor program project (must contain Anchor.toml or a
        /// standalone Cargo.toml program crate).
        #[arg(default_value = ".")]
        program: PathBuf,
    },
    /// Mutate, build each mutant, and run the suite on LiteSVM
    Run {
        /// Path to the Anchor program project.
        #[arg(default_value = ".")]
        program: PathBuf,
        /// Output directory for the produced report files.
        #[arg(short, long, default_value = "target/mutanchor")]
        out: PathBuf,
        /// Per-mutant timeout in seconds.
        #[arg(long, default_value_t = 180)]
        timeout: u64,
        /// Skip an operator (repeatable). e.g. --skip comparison_flip
        #[arg(long, value_name = "OPERATOR")]
        skip: Vec<String>,
        /// Only emit the mutation set without executing (dry-run).
        #[arg(long)]
        dry_run: bool,
    },
    /// Emit the mutation report
    Report {
        /// Path to the JSON report produced by `run`.
        #[arg(default_value = "target/mutanchor/report.json")]
        report: PathBuf,
        /// Output HTML file (defaults to report.html next to the JSON).
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// CI mode: fail when surviving mutants exceed a threshold
    Ci {
        /// Path to the program project.
        #[arg(default_value = ".")]
        program: PathBuf,
        /// Maximum allowed surviving mutants before exit code 1.
        #[arg(long, default_value_t = 0)]
        max_survivors: usize,
        /// Only consider survivors (killed/build-failed/timeout do not count).
        #[arg(long, default_value_t = true)]
        survivors_only: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli) {
        eprintln!("mutanchor: error: {e:#}");
        std::process::exit(1);
    }
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init { program } => {
            let manifest = init::scan(&program)?;
            let mut out = String::new();
            out.push_str(&format!(
                "program dir: {}\n",
                manifest.program_dir.display()
            ));
            out.push_str(&format!(
                "instructions ({}):\n",
                manifest.instructions.len()
            ));
            for ins in &manifest.instructions {
                out.push_str(&format!(
                    "  {:>3}  {:<24} {}\n",
                    ins.range.start, ins.name, ins.file
                ));
            }
            print!("{out}");
            Ok(())
        }
        Command::Run {
            program,
            out,
            timeout,
            skip,
            dry_run,
        } => engine::run(&program, &out, Duration::from_secs(timeout), &skip, dry_run),
        Command::Report { report, html } => {
            let r = engine::load_report(&report)?;
            let html_path = html.unwrap_or_else(|| report.with_file_name("report.html"));
            if let Some(parent) = html_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create {}", parent.display()))?;
                }
            }
            let rendered = report::to_html(&r)?;
            std::fs::write(&html_path, rendered)
                .with_context(|| format!("write {}", html_path.display()))?;
            println!("wrote {}", html_path.display());
            println!(
                "score: {:.1}% ({} mutants)",
                r.score() * 100.0,
                r.mutants_total
            );
            Ok(())
        }
        Command::Ci {
            program,
            max_survivors,
            survivors_only,
        } => engine::ci(&program, max_survivors, survivors_only),
    }
}
