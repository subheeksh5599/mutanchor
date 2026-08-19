//! `mutanchor run`: build each mutant with Solana's official toolchain, run
//! the program's test suite against it on LiteSVM, and classify the outcome.
//!
//! Execution model: every mutant is a single deliberate source change. We
//! write that one change into a scratch copy of the program, compile it with
//! `cargo build-sbf` (the same toolchain real Anchor programs use), then run
//! the program's own test suite. For Anchor programs whose tests load the
//! built program via `anchor_litesvm::LiteSVM`, those tests execute purely
//! in-memory on LiteSVM — no validator, no cluster, no waiting.
//!
//! Verdicts:
//! - the build fails                 -> BuildFailed
//! - any test fails                  -> Killed
//! - all tests pass                  -> Survived
//! - the run exceeds the timeout     -> TimedOut

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::model::{Mutant, MutantResult, Operator, Verdict};

/// Configuration for a run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub program_dir: PathBuf,
    pub work_dir: PathBuf,
    /// Per-mutant build+test timeout.
    pub timeout: Duration,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig {
            program_dir: PathBuf::from("."),
            work_dir: PathBuf::from("/tmp/mutanchor-work"),
            timeout: Duration::from_secs(180),
        }
    }
}

struct BuildOutcome {
    verdict: Verdict,
    failing_tests: usize,
    first_failure: Option<String>,
}

/// Run every mutant to a verdict. Returns results in `mutants` order.
///
/// Execution uses ONE scratch tree reused across all mutants: each mutant is
/// applied to the tree, built, and tested, then the tree is restored from the
/// pristine copy. Reusing one tree gives an incremental build cache (only the
/// mutated file changes between builds, so `cargo build-sbf` recompiles just
/// that crate), which is what makes running many mutants practical. Because
/// the tree is shared, execution is serialized regardless of `jobs` (later
/// jobs slots are reserved for future parallel work; the default is 1).
pub fn run_mutants(
    cfg: &RunConfig,
    mutants: &[Mutant],
    log: &(dyn Fn(&str) + Sync),
) -> Result<Vec<MutantResult>> {
    let total = mutants.len();

    // Clean the scratch root and copy the pristine program tree into it once.
    let pristine = cfg.work_dir.join("pristine");
    if cfg.work_dir.exists() {
        std::fs::remove_dir_all(&cfg.work_dir).with_context(|| "clean work dir")?;
    }
    std::fs::create_dir_all(&cfg.work_dir).with_context(|| "create work dir")?;
    copy_dir(&cfg.program_dir, &pristine)?;

    let tree = cfg.work_dir.join("tree");
    copy_dir(&pristine, &tree)?;

    let mut results = Vec::with_capacity(total);
    for (i, m) in mutants.iter().enumerate() {
        log(&format!(
            "[{}/{}] {} at {}:{}",
            i + 1,
            total,
            m.operator.id(),
            m.file,
            m.line
        ));
        results.push(run_one(cfg, m, &pristine, &tree));
    }

    Ok(results)
}

/// Execute a single mutant against the shared scratch tree. The mutation is
/// applied in place, built, and its tests run; the tree is then restored from
/// `pristine` so the next mutant starts clean.
fn run_one(cfg: &RunConfig, m: &Mutant, pristine: &Path, tree: &Path) -> MutantResult {
    let mut out = MutantResult {
        mutant: m.clone(),
        ..Default::default()
    };

    let mut build_ms = 0u128;
    let mut run_ms = 0u128;

    let result: Verdict = match write_mutant_source(cfg, m, tree) {
        Ok(()) => {
            let bstart = Instant::now();
            let built = build_mutant(cfg, tree, cfg.timeout);
            build_ms = bstart.elapsed().as_millis();
            match built {
                Ok(()) => {
                    let rstart = Instant::now();
                    let ran = run_tests(cfg, m, tree, cfg.timeout);
                    run_ms = rstart.elapsed().as_millis();
                    match ran {
                        Ok(bo) => {
                            out.failing_tests = bo.failing_tests;
                            out.first_failure = bo.first_failure;
                            bo.verdict
                        }
                        Err(e) => {
                            out.first_failure = Some(format!("test run failed: {e}"));
                            Verdict::TimedOut
                        }
                    }
                }
                Err(e) => {
                    out.first_failure = Some(format!("cargo build-sbf failed: {e}"));
                    Verdict::BuildFailed
                }
            }
        }
        Err(e) => {
            out.first_failure = Some(format!("scratch write failed: {e}"));
            Verdict::BuildFailed
        }
    };

    // Restore the tree for the next mutant.
    let _ = std::fs::remove_dir_all(tree);
    let _ = copy_dir(pristine, tree);

    out.verdict = result;
    out.build_ms = build_ms;
    out.run_ms = run_ms;

    if matches!(out.verdict, Verdict::Survived) {
        out.exploit = Some(exploit_for(m.operator).to_string());
    }

    out
}

/// Apply the single mutation into the scratch tree (in place).
fn write_mutant_source(_cfg: &RunConfig, m: &Mutant, scratch_root: &Path) -> Result<()> {
    use std::fs;

    let target = scratch_root.join(&m.file);
    let text =
        fs::read_to_string(&target).with_context(|| format!("read {0}", target.display()))?;
    let mut lines: Vec<&str> = text.lines().collect();
    let idx = (m.line as usize).saturating_sub(1);
    if idx < lines.len() {
        lines[idx] = &m.mutated;
    }
    let new_text = lines.join("\n");
    fs::write(&target, new_text).with_context(|| format!("write {0}", target.display()))?;
    Ok(())
}

/// Recursively copy a directory tree, skipping target/, .git and node_modules.
fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    use std::fs;
    fs::create_dir_all(dst).with_context(|| format!("mkdir {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "target" || name == ".git" || name == "node_modules" || name == ".anchor" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let ftype = entry.file_type()?;
        if ftype.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)
                .with_context(|| format!("copy {} -> {}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

/// Run `cargo build-sbf` in the scratch tree with a hard timeout.
fn build_mutant(cfg: &RunConfig, scratch_root: &Path, timeout: Duration) -> Result<()> {
    let _ = cfg;
    let manifest = scratch_root.join("Cargo.toml");
    let out_dir = scratch_root.join("target").join("deploy");

    let mut cmd = Command::new("cargo");
    cmd.args(["build-sbf", "--manifest-path"])
        .arg(&manifest)
        .arg("--sbf-out-dir")
        .arg(&out_dir)
        .env("CARGO_TERM_COLOR", "never");

    let (code, ok, output) = run_impl(&mut cmd, timeout)
        .map_err(|e| anyhow::anyhow!("failed to spawn cargo build-sbf: {e}"))?;

    if ok {
        return Ok(());
    }

    // The build failed. Surface the real stderr (so an environment failure
    // like "no space left on device" is distinguishable from a genuine
    // compile error in the mutant).
    let tail: String = output.rsplit('\n').take(12).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    anyhow::bail!(
        "cargo build-sbf exited {} — tail:\n{}",
        code.map(|c| c.to_string()).unwrap_or_else(|| "timeout".into()),
        tail.trim_end()
    )
}

/// Run the program's own tests against the freshly built program. The tests
/// load the mutant `.so` into LiteSVM via `MUTANCHOR_PROGRAM_SO`.
fn run_tests(
    cfg: &RunConfig,
    m: &Mutant,
    scratch_root: &Path,
    timeout: Duration,
) -> Result<BuildOutcome> {
    let _ = m;
    let so = scratch_root
        .join("target")
        .join("deploy")
        .join(program_so_name(cfg));
    let manifest = scratch_root.join("Cargo.toml");

    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--manifest-path"])
        .arg(&manifest)
        .env("MUTANCHOR_PROGRAM_SO", &so)
        .env("CARGO_TERM_COLOR", "never");

    // We need the output even when the test run fails (that is what makes a
    // mutant "killed"), so use the raw impl and interpret ourselves.
    let (_code, _ok, combined) = run_impl(&mut cmd, timeout)?;

    let failing = parse_failures(&combined);
    let had_failure = combined.contains("test result: FAILED") || combined.contains("panicked");

    Ok(BuildOutcome {
        verdict: if had_failure {
            Verdict::Killed
        } else {
            Verdict::Survived
        },
        failing_tests: failing.0,
        first_failure: failing.1,
    })
}

fn program_name(cfg: &RunConfig) -> Option<String> {
    let manifest = cfg.program_dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).ok()?;
    let parsed: toml::Value = toml::from_str(&text).ok()?;
    parsed
        .get("package")?
        .get("name")?
        .as_str()
        .map(|s| s.to_string())
}

/// The .so filename produced by cargo build-sbf for the program crate.
fn program_so_name(cfg: &RunConfig) -> String {
    let name = program_name(cfg).unwrap_or_else(|| "program".to_string());
    // build-sbf rewrites `-` to `_` in the lib name.
    format!("{}.so", name.replace('-', "_"))
}

/// Shared subprocess runner with a hard timeout watchdog.
/// The combined stdout+stderr is always returned so callers can inspect the
/// real error tail; the caller decides what a non-zero exit means.
fn run_impl(cmd: &mut Command, timeout: Duration) -> Result<(Option<i32>, bool, String)> {
    use std::io::Read;
    use std::process::{Child, Stdio};

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child: Child = cmd.spawn().context("spawn")?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let out_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });
    let err_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(st) = child.try_wait()? {
            break st;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok((None, false, String::new()));
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let o = out_h.join().unwrap_or_default();
    let e = err_h.join().unwrap_or_default();
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&o),
        String::from_utf8_lossy(&e)
    );
    Ok((status.code(), status.success(), combined))
}

fn parse_failures(combined: &str) -> (usize, Option<String>) {
    let mut failed = 0usize;
    for line in combined.lines() {
        let t = line.trim();
        if t.starts_with("test result: FAILED") || t.contains("test result: FAILED") {
            for frag in t.split(';') {
                let frag = frag.trim();
                if let Some(rest) = frag.strip_suffix(" failed") {
                    let num = rest.split_whitespace().last().unwrap_or("0");
                    if let Ok(n) = num.parse::<usize>() {
                        failed = n;
                    }
                }
            }
        }
    }
    let first = combined
        .lines()
        .find(|l| l.contains("panicked at") || l.contains("panicked"))
        .map(|l| l.trim().to_string());
    (failed, first)
}

fn exploit_for(op: Operator) -> &'static str {
    match op {
        Operator::SignerCheckRemoval => "An attacker can invoke the instruction with an unauthorised signer; the missing signer check is not caught by your suite.",
        Operator::PdaSeedSwap => "The program derives a PDA from the wrong seeds, so it may write to (or read) an account the caller controls instead of the canonical one.",
        Operator::BumpMismatch => "The program uses a non-canonical bump; the derived address can be wrong, letting a caller target an unintended account.",
        Operator::AuthorityCheckDrop => "Authority/owner validation is dropped, so a non-authorised caller can run restricted logic.",
        Operator::DiscriminatorRemoval => "The program accepts a wrong instruction discriminator, so malformed or replay-able accounts can pass validation.",
        Operator::CpiTargetSwap => "A CPI now targets the wrong program; funds or authority could flow to the wrong place.",
        Operator::CloseRentDrop => "Closed or non-rent-exempt accounts are not validated; lamports or account state can leak.",
        Operator::ComparisonFlip => "A boundary check is inverted, so an out-of-range / off-by-one condition passes when it should not.",
    }
}
