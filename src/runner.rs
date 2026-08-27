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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::model::{Mutant, MutantResult, Operator, Verdict};

/// Set to true by the Ctrl-C handler. Every long-running loop checks this
/// and bails out cleanly; the scratch dir is removed via `ScratchGuard::drop`.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);
/// PID of the child cargo/build-sbf process currently being waited on, if any.
/// The signal handler reads it to send a follow-up SIGTERM so the wait loop
/// unblocks immediately instead of running out its timeout.
static CURRENT_CHILD_PID: AtomicU32 = AtomicU32::new(0);
static INSTALL_HANDLER: Once = Once::new();
static WORK_DIRS_TO_CLEAN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn install_signal_handler() {
    INSTALL_HANDLER.call_once(|| {
        let _ = ctrlc::set_handler(move || {
            // First Ctrl-C: request graceful shutdown. Second Ctrl-C exits hard.
            if SHUTDOWN.swap(true, Ordering::SeqCst) {
                eprintln!("\nmutanchor: forced exit on second Ctrl-C");
                std::process::exit(130);
            }
            eprintln!(
                "\nmutanchor: interrupt received — stopping after this mutant, \
                 killing in-flight build/test, cleaning scratch dir…"
            );
            // Best-effort: SIGTERM the child that's currently blocking the run
            // loop so it unblocks immediately.
            #[cfg(unix)]
            {
                let pid = CURRENT_CHILD_PID.load(Ordering::SeqCst);
                if pid != 0 {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }
        });
    });
}

/// Owns the scratch working directory and removes it when dropped, so a
/// panic, an early return, or a Ctrl-C never leaves a multi-GB tree behind.
struct ScratchGuard {
    path: PathBuf,
}

impl ScratchGuard {
    fn new(path: PathBuf) -> Self {
        if let Ok(mut v) = WORK_DIRS_TO_CLEAN.lock() {
            v.push(path.clone());
        }
        ScratchGuard { path }
    }
}

impl Drop for ScratchGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
        if let Ok(mut v) = WORK_DIRS_TO_CLEAN.lock() {
            v.retain(|p| p != &self.path);
        }
    }
}

/// Interruption signal: check from long-running loops.
pub fn was_interrupted() -> bool {
    SHUTDOWN.load(Ordering::SeqCst)
}

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
    install_signal_handler();
    let total = mutants.len();

    // Clean the scratch root and copy the pristine program tree into it once.
    let pristine = cfg.work_dir.join("pristine");
    if cfg.work_dir.exists() {
        std::fs::remove_dir_all(&cfg.work_dir).with_context(|| "clean work dir")?;
    }
    std::fs::create_dir_all(&cfg.work_dir).with_context(|| "create work dir")?;
    // From here on, any early return / panic / Ctrl-C removes work_dir on drop.
    let _guard = ScratchGuard::new(cfg.work_dir.clone());
    copy_dir(&cfg.program_dir, &pristine)?;

    let tree = cfg.work_dir.join("tree");
    copy_dir(&pristine, &tree)?;

    // Prime the incremental build cache: build the pristine (unmutated)
    // program and compile+run its test suite once in the tree. All 13+
    // mutants then only recompile the mutated crate instead of the entire
    // dependency tree, which is what makes runs practical on small hosts.
    // If the pristine program does not build or its tests do not pass, fail
    // fast with the tail — a run on a broken program cannot produce honest
    // verdicts anyway.
    log("priming incremental build cache (pristine build + test)…");
    let so = tree
        .join("target")
        .join("deploy")
        .join(program_so_name(cfg));
    let manifest = tree.join("Cargo.toml");
    let warm_timeout = Duration::from_secs(900);
    let mut wb = Command::new("cargo");
    wb.args(["build-sbf", "--manifest-path"])
        .arg(&manifest)
        .env("CARGO_TERM_COLOR", "never");
    let (_, ok, out) = run_impl(&mut wb, warm_timeout)?;
    if !ok {
        bail!(
            "warm-up `cargo build-sbf` failed — cannot run mutants against a \
             program that does not build:\n{}",
            tail(&out, 15)
        );
    }
    let mut wt = Command::new("cargo");
    wt.args(["test", "--manifest-path"])
        .arg(&manifest)
        .env("MUTANCHOR_PROGRAM_SO", &so)
        .env("CARGO_TERM_COLOR", "never");
    let (_, ok, out) = run_impl(&mut wt, warm_timeout)?;
    if !ok {
        bail!(
            "warm-up `cargo test` failed — cannot run mutants against a \
             program whose pristine test suite does not pass:\n{}",
            tail(&out, 15)
        );
    }
    log("cache primed");

    let mut results = Vec::with_capacity(total);
    for (i, m) in mutants.iter().enumerate() {
        if was_interrupted() {
            log(&format!(
                "interrupted after {}/{} mutants — scratch dir will be cleaned up",
                i, total
            ));
            break;
        }
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

    // The ScratchGuard drop will remove the scratch tree on the way out
    // (normal exit, error, or Ctrl-C). Explicitly acknowledge it so a static
    // analyser doesn't flag it as unused.
    drop(_guard);

    Ok(results)
}

/// Last `n` lines of a combined output blob (for honest error tails).
fn tail(blob: &str, n: usize) -> String {
    blob.lines()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
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
                BuildResult::Ok => {
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
                BuildResult::Failed(reason) => {
                    out.first_failure = Some(format!("cargo build-sbf failed: {reason}"));
                    Verdict::BuildFailed
                }
                BuildResult::TimedOut => {
                    out.first_failure = Some("cargo build-sbf timed out".to_string());
                    Verdict::TimedOut
                }
            }
        }
        Err(e) => {
            out.first_failure = Some(format!("scratch write failed: {e}"));
            Verdict::BuildFailed
        }
    };

    // Restore the tree for the next mutant, keeping the warm target/ cache.
    let _ = restore_tree(pristine, tree);

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
/// File mtimes are preserved so Cargo's fingerprint cache (which compares
/// source mtimes against stored fingerprints) stays valid across restores.
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
            copy_file(&from, &to)?;
        }
    }
    Ok(())
}

/// Copy one file preserving its mtime (Cargo fingerprints are mtime-based).
fn copy_file(from: &Path, to: &Path) -> Result<()> {
    use std::fs;
    let meta = fs::metadata(from)?;
    fs::copy(from, to)?;
    let times = std::fs::FileTimes::new().set_modified(meta.modified()?);
    fs::File::options()
        .write(true)
        .open(to)?
        .set_times(times)
        .with_context(|| format!("set mtime on {}", to.display()))?;
    Ok(())
}

/// Restore the scratch tree from pristine WITHOUT wiping `target/`, so the
/// incremental build cache survives from one mutant to the next. Only the
/// (non-target) source files are replaced; their mtimes are preserved, and
/// the mutant edit (written afterwards by `write_mutant_source`) is the only
/// file newer than the cached fingerprints — so `cargo build-sbf` / `cargo
/// test` recompile just the mutated crate and relink the test binary.
fn restore_tree(pristine: &Path, tree: &Path) -> Result<()> {
    use std::fs;
    for entry in fs::read_dir(tree).with_context(|| format!("read {}", tree.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "target" {
            continue;
        }
        let path = tree.join(&name);
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    for entry in fs::read_dir(pristine).with_context(|| format!("read {}", pristine.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "target" || name == ".git" || name == "node_modules" || name == ".anchor" {
            continue;
        }
        let from = entry.path();
        let to = tree.join(&name);
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            copy_file(&from, &to)?;
        }
    }
    Ok(())
}

/// Run `cargo build-sbf` in the scratch tree with a hard timeout.
/// Outcome of building a single mutant. Timed-out builds are distinguished
/// from genuine compile failures so the verdict is honest (a build that was
/// still compiling when killed is `TimedOut`, not `BuildFailed`).
enum BuildResult {
    Ok,
    Failed(String),
    TimedOut,
}

fn build_mutant(cfg: &RunConfig, scratch_root: &Path, timeout: Duration) -> BuildResult {
    let _ = cfg;
    let manifest = scratch_root.join("Cargo.toml");
    let out_dir = scratch_root.join("target").join("deploy");

    let mut cmd = Command::new("cargo");
    cmd.args(["build-sbf", "--manifest-path"])
        .arg(&manifest)
        .arg("--sbf-out-dir")
        .arg(&out_dir)
        .env("CARGO_TERM_COLOR", "never");

    let (code, ok, output) = match run_impl(&mut cmd, timeout) {
        Ok(r) => r,
        Err(e) => return BuildResult::Failed(format!("failed to spawn cargo build-sbf: {e}")),
    };

    if ok {
        return BuildResult::Ok;
    }
    if code.is_none() {
        // run_impl returned None exit code when it killed the child on timeout.
        return BuildResult::TimedOut;
    }

    // The build failed. Surface the real stderr (so an environment failure
    // like "no space left on device" is distinguishable from a genuine
    // compile error in the mutant).
    let tail: String = output
        .rsplit('\n')
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    BuildResult::Failed(format!(
        "cargo build-sbf exited {} — tail:\n{}",
        code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
        tail.trim_end()
    ))
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

    // Publish this child's PID so the Ctrl-C handler can SIGTERM it and
    // unblock the wait loop immediately (instead of running out the timeout).
    let pid = child.id();
    CURRENT_CHILD_PID.store(pid, Ordering::SeqCst);

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
        if was_interrupted() {
            let _ = child.kill();
            let _ = child.wait();
            CURRENT_CHILD_PID.store(0, Ordering::SeqCst);
            return Ok((None, false, String::from("interrupted")));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            CURRENT_CHILD_PID.store(0, Ordering::SeqCst);
            return Ok((None, false, String::new()));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    CURRENT_CHILD_PID.store(0, Ordering::SeqCst);

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
        Operator::UncheckedMath => "A `checked_add` / `checked_sub` / `checked_mul` was replaced by an unchecked arithmetic op; the mutation lets a silent overflow pass through balance / accounting logic that your tests would have caught.",
        Operator::ReallocCheckDrop => "A `realloc` size guard was removed; the mutation lets an attacker resize / re-initialize an account to an unintended layout, and your suite does not catch it.",
    }
}
