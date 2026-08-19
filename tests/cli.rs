//! CLI integration tests.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_prints_the_four_subcommands() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("report"))
        .stdout(predicate::str::contains("ci"));
}

#[test]
fn version_flag_prints_semver() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn init_scan_discovers_instructions_in_fixture() {
    // The demo/fixture program has two handlers (deposit, withdraw).
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.args(["init", "demo/fixture"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deposit"))
        .stdout(predicate::str::contains("withdraw"));
}

#[test]
fn dry_run_lists_mutants_without_executing() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.args(["run", "demo/fixture", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generated"))
        .stdout(predicate::str::contains("pda_seed_swap"))
        .stdout(predicate::str::contains("comparison_flip"));
}
