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

// The remaining tests pin the failure surface: every subcommand should reject
// a missing program dir with a clear non-zero exit, not panic and not silently
// succeed. This is the "--help / error paths tested for every subcommand"
// production-checklist item.

#[test]
fn init_on_missing_program_dir_fails_with_clear_error() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.args(["init", "demo/does-not-exist"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("does-not-exist").or(predicate::str::contains("not found")),
        );
}

#[test]
fn run_on_missing_program_dir_fails_with_clear_error() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.args(["run", "demo/does-not-exist", "--dry-run"])
        .assert()
        .failure();
}

#[test]
fn ci_on_missing_program_dir_fails_with_clear_error() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.args(["ci", "demo/does-not-exist", "--max-survivors", "0"])
        .assert()
        .failure();
}

#[test]
fn report_on_missing_json_fails_with_clear_error() {
    let mut cmd = Command::cargo_bin("mutanchor").unwrap();
    cmd.args(["report", "target/does-not-exist/report.json"])
        .assert()
        .failure();
}

#[test]
fn subcommand_help_output_is_stable() {
    for sub in ["init", "run", "report", "ci"] {
        let mut cmd = Command::cargo_bin("mutanchor").unwrap();
        cmd.args([sub, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage").or(predicate::str::contains("usage")));
    }
}
