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
    Init,
    /// Mutate, build each mutant, and run the suite on LiteSVM
    Run,
    /// Emit the mutation report
    Report,
    /// CI mode: fail when surviving mutants exceed a threshold
    Ci,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => println!("init: not implemented"),
        Command::Run => println!("run: not implemented"),
        Command::Report => println!("report: not implemented"),
        Command::Ci => println!("ci: not implemented"),
    }
}
