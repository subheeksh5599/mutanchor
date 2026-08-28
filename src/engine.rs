//! Orchestration: turn a program directory into a finished Report, and gate
//! the build in CI. This is where the deterministic engine (ops) meets the
//! executor (runner) and the renderer (report).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::model::{InstructionScore, Mutant, MutantResult, Operator, Report, Verdict};
use crate::{init, ops, report, runner};

/// Full `run` pipeline.
pub fn run(
    program: &Path,
    out: &Path,
    timeout: Duration,
    skip: &[String],
    dry_run: bool,
    jobs: usize,
    test_features: Option<&str>,
) -> Result<()> {
    // 1. Discover the program and its instructions.
    let manifest = init::scan(program)?;
    let program_dir = manifest.program_dir.clone();

    // 2. Collect the source files we will mutate (the ones that define the
    //    program's instructions, plus lib.rs).
    let mut files: Vec<(String, String)> = Vec::new();
    {
        let lib_path = program_dir.join("src").join("lib.rs");
        let text = std::fs::read_to_string(&lib_path)
            .with_context(|| format!("read {}", lib_path.display()))?;
        files.push(("src/lib.rs".to_string(), text));
    }
    // Include inline module files (e.g. src/instructions/*.rs) that are part
    // of the program. Discovered via `mod` statements is complex; for a first
    // real pass we mutate lib.rs and any top-level `src/*.rs` files that exist.
    if let Ok(rd) = std::fs::read_dir(program_dir.join("src")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "rs").unwrap_or(false) {
                let rel = format!("src/{}", p.file_name().unwrap().to_string_lossy());
                if rel != "src/lib.rs" {
                    let text = std::fs::read_to_string(&p)
                        .with_context(|| format!("read {}", p.display()))?;
                    files.push((rel, text));
                }
            }
        }
    }

    // 3. Instruction line ranges for attribution.
    let instr_ranges: Vec<(String, std::ops::Range<usize>)> = manifest
        .instructions
        .iter()
        .map(|i| (i.name.clone(), i.range.clone()))
        .collect();

    // 4. Generate all mutants across the mutated files.
    let skip_set: Vec<Operator> = skip
        .iter()
        .filter_map(|s| {
            Operator::ALL
                .iter()
                .find(|o| {
                    o.id() == s
                        || format!("{:?}", o).to_lowercase().replace('_', "")
                            == s.to_lowercase().replace('_', "")
                })
                .copied()
        })
        .collect();

    let mut generated: Vec<Mutant> = Vec::new();
    for (rel, text) in &files {
        let ms = ops::generate(text, rel, &instr_ranges);
        generated.extend(ms);
    }

    // 5. Drop equivalent / meaningless mutants.
    let (mutants, dropped) = dedup_equivalents(generated, &skip_set);

    println!(
        "generated {} mutants ({} equivalent/duplicate dropped) across {} file(s)",
        mutants.len(),
        dropped,
        files.len()
    );
    if mutants.is_empty() {
        anyhow::bail!("no mutants generated — nothing to run. Check the program has Anchor instruction handlers.");
    }

    // 6. Dry-run: just list them.
    if dry_run {
        for m in &mutants {
            println!(
                "  #{} {:24} {}:{:<4} {}  ->  {}",
                m.id,
                m.operator.id(),
                m.file,
                m.line,
                m.original.trim(),
                m.mutated.trim()
            );
        }
        return Ok(());
    }

    // 7. Execute.
    let cfg = runner::RunConfig {
        program_dir: program_dir.clone(),
        work_dir: PathBuf::from("/tmp/mutanchor-work"),
        timeout,
        jobs: jobs.max(1),
        test_features: test_features.map(str::to_string),
    };
    let results = runner::run_mutants(&cfg, &mutants, &|msg| println!("{msg}"))?;

    // 8. Aggregate into a Report.
    let report = build_report(program_dir, &results, dropped);

    // 9. Write report files + a JSON embed for the frontend dashboard.
    std::fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;
    let (json_path, html_path) = report::write_files(&report, out)?;

    // 10. Summary summary.
    print_summary(&report);
    println!("report.json:  {}", json_path.display());
    println!("report.html:  {}", html_path.display());
    println!("dashboard.json: {}", out.join("dashboard.json").display());
    Ok(())
}

/// Remove meaningless mutants: exact duplicates (same operator+location+text)
/// and mutants whose mutated text is identical to the original after trimming
/// (no actual change), plus any in the skip set.
fn dedup_equivalents(mutants: Vec<Mutant>, skip: &[Operator]) -> (Vec<Mutant>, usize) {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut kept: Vec<Mutant> = Vec::new();
    let mut dropped = 0usize;
    for m in mutants {
        if skip.contains(&m.operator) {
            dropped += 1;
            continue;
        }
        // Meaningless: nothing actually changed.
        if m.original.trim() == m.mutated.trim() {
            dropped += 1;
            continue;
        }
        let key = format!("{}:{}:{}:{}", m.operator.id(), m.file, m.line, m.mutated);
        if !seen.insert(key) {
            dropped += 1;
            continue;
        }
        kept.push(m);
    }
    // Renumber contiguous ids.
    for (i, m) in kept.iter_mut().enumerate() {
        m.id = i;
    }
    (kept, dropped)
}

fn build_report(program_dir: PathBuf, results: &[MutantResult], dropped: usize) -> Report {
    let prog = init_name(&program_dir);
    let mut killed = 0;
    let mut survived = 0;
    let mut build_failed = 0;
    let mut timed_out = 0;
    for r in results {
        match r.verdict {
            Verdict::Killed => killed += 1,
            Verdict::Survived => survived += 1,
            Verdict::BuildFailed => build_failed += 1,
            Verdict::TimedOut => timed_out += 1,
        }
    }

    // Per-instruction aggregation.
    let mut by_instr: std::collections::BTreeMap<String, InstructionScore> =
        std::collections::BTreeMap::new();
    for r in results {
        let name = r
            .mutant
            .instruction
            .clone()
            .unwrap_or_else(|| "(unknown)".to_string());
        let entry = by_instr
            .entry(name.clone())
            .or_insert_with(|| InstructionScore {
                instruction: name.clone(),
                killed: 0,
                survived: 0,
                build_failed: 0,
                timed_out: 0,
                total: 0,
            });
        entry.total += 1;
        match r.verdict {
            Verdict::Killed => entry.killed += 1,
            Verdict::Survived => entry.survived += 1,
            Verdict::BuildFailed => entry.build_failed += 1,
            Verdict::TimedOut => entry.timed_out += 1,
        }
    }
    let per_instruction: Vec<InstructionScore> = by_instr.into_values().collect();
    let survivors: Vec<MutantResult> = results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Survived))
        .cloned()
        .collect();

    Report {
        program: prog,
        generated_at: now_iso(),
        mutants_total: results.len(),
        killed,
        survived,
        build_failed,
        timed_out,
        dropped_equivalent: dropped,
        per_instruction,
        survivors,
        mutants: results.to_vec(),
    }
}

fn init_name(program_dir: &Path) -> String {
    let manifest = program_dir.join("Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(&manifest) {
        if let Ok(v) = text.parse::<toml::Value>() {
            if let Some(n) = v
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                return n.to_string();
            }
        }
    }
    program_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "program".into())
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple UTC-ish timestamp (no tz crate dependency).
    let days = secs / 86400;
    let _ = days;
    // Approximate with chrono-free formatting using a fixed offset via git-style.
    format_epoch(secs)
}

/// Minimal civil-date formatter for epoch seconds (UTC). Avoids pulling a
/// tz crate.
fn format_epoch(secs: u64) -> String {
    let days = secs / 86400;
    let rem = secs % 86400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    // Algorithm for civil date from a day number (Howard Hinnant).
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn print_summary(r: &Report) {
    println!(
        "mutation score: {:.1}% ({killed} killed, {survived} survived, {bf} build-failed, {to} timeout)",
        r.score() * 100.0,
        killed = r.killed,
        survived = r.survived,
        bf = r.build_failed,
        to = r.timed_out,
    );
}

/// Load a saved JSON report.
pub fn load_report(path: &Path) -> Result<Report> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

/// CI gate: exit non-zero when the survivor count is too high.
pub fn ci(program: &Path, max_survivors: usize, _survivors_only: bool) -> Result<()> {
    let _manifest = init::scan(program)?;
    // Reuse the run pipeline but only evaluate survivors by running a dry
    // generation (no execution) would be wrong — CI must reflect a real run,
    // so we re-run the full pipeline into a temp report and gate on it.
    let out = std::env::temp_dir().join(format!("mutanchor-ci-{}", std::process::id()));
    run(program, &out, Duration::from_secs(180), &[], false, 1, None)?;
    let r = load_report(&out.join("report.json"))?;
    let survivors = r.survived;
    println!("surviving mutants: {survivors} (max allowed: {max_survivors})");
    if ci_verdict(survivors, max_survivors) {
        anyhow::bail!("CI gate: {survivors} surviving mutants exceeds max of {max_survivors}");
    }
    Ok(())
}

/// Pure gate decision: true means the build must fail (survivors exceed the
/// allowed maximum), false means it passes.
fn ci_verdict(survivors: usize, max_survivors: usize) -> bool {
    survivors > max_survivors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_fails_only_when_survivors_exceed_max() {
        assert!(!ci_verdict(0, 0), "0 survivors with max 0 must pass");
        assert!(!ci_verdict(1, 1), "at max must pass");
        assert!(!ci_verdict(0, 1), "under max must pass");
        assert!(ci_verdict(2, 1), "over max must fail");
        assert!(ci_verdict(3, 0), "any survivor with max 0 must fail");
    }
}
