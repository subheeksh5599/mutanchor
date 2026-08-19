//! `mutanchor report`: render a run's `Report` as JSON and as a static HTML
//! scorecard that the demo site's `/dashboard` panel renders.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::json;

use crate::model::{Report, Verdict};

/// Render the report as pretty JSON (CI-consumable).
pub fn to_json(report: &Report) -> Result<String> {
    serde_json::to_string_pretty(report).context("serialize report to JSON")
}

/// Render the report as a self-contained HTML scorecard. No network assets;
/// safe to publish to the Vercel report panel.
pub fn to_html(report: &Report) -> Result<String> {
    let rows: Vec<String> = report
        .mutants
        .iter()
        .map(|r| {
            let _v = r.verdict.id();
            let cls = match r.verdict {
                Verdict::Killed => "killed",
                Verdict::Survived => "survived",
                Verdict::BuildFailed => "failed",
                Verdict::TimedOut => "timeout",
            };
            let instr = r.mutant.instruction.as_deref().unwrap_or("-");
            let expl = r.exploit.as_deref().unwrap_or("&ndash;");
            format!(
                r#"<tr class="{cls}"><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td class="loc"><code>{}:{}</code></td><td><code>{}</code></td><td><code>{}</code></td><td class="inline-sum">{}</td></tr>"#,
                r.mutant.id,
                r.mutant.operator.id(),
                instr,
                escape_html(&r.mutant.file),
                r.mutant.line,
                escape_html(&r.mutant.original),
                escape_html(&r.mutant.mutated),
                escape_html(expl),
            )
        })
        .collect();

    let inst_rows: Vec<String> = report
        .per_instruction
        .iter()
        .map(|s| {
            format!(
                r#"<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.1}%</td></tr>"#,
                escape_html(&s.instruction),
                s.killed,
                s.survived,
                s.build_failed,
                s.timed_out,
                s.total,
                s.score() * 100.0,
            )
        })
        .collect();

    let summary = if report.is_empty() {
        "<p class=\"empty\">No run yet. Run <code>mutanchor run</code> on an Anchor \
         program and publish its report here — this panel stays empty until a real \
         run exists. Nothing on this site is sample data.</p>"
            .to_string()
    } else {
        format!(
            "<p class=\"score\">Mutation score <strong>{:.1}%</strong></p>\
             <p>{killed} killed &middot; {survived} survived &middot; {bf} build-failed \
             &middot; {to} timeout &middot; {eq} equivalent dropped</p>",
            report.score() * 100.0,
            killed = report.killed,
            survived = report.survived,
            bf = report.build_failed,
            to = report.timed_out,
            eq = report.dropped_equivalent,
        )
    };

    let html = format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Mutanchor — report</title>
<style>
:root{{color-scheme:light}}
body{{margin:0;font-family:ui-sans-serif,system-ui,sans-serif;background:#fafafa;color:#111;line-height:1.5}}
.wrap{{max-width:1100px;margin:0 auto;padding:3rem 1.5rem}}
h1{{font-size:1.6rem;margin:0 0 .2rem}}
.sub{{color:#666}}
.score{{font-size:1.1rem;margin:1rem 0}}
.score strong{{color:#0a7}}
table{{width:100%;border-collapse:collapse;margin-top:1.5rem;font-size:.85rem}}
th,td{{text-align:left;padding:.5rem .6rem;border-bottom:1px solid #e5e5e5;vertical-align:top}}
th{{background:#f0f0f0;font-weight:600}}
code{{font-family:ui-monospace,monospace;font-size:.8rem;background:#f1f1f1;padding:.1rem .3rem;border-radius:4px}}
.loc code{{white-space:nowrap}}
tr.killed td:first-child{{background:#e8f7ef}}
tr.survived td:first-child{{background:#fde8e8}}
tr.survived{{outline:1px solid #f5c2c2}}
tr.failed td:first-child{{background:#f6f6f6}}
tr.timeout td:first-child{{background:#fff7e6}}
.inline-sum{{color:#444;max-width:38ch}}
.empty{{color:#888;font-style:italic}}
.badge{{display:inline-block;padding:.15rem .5rem;border-radius:999px;background:#0a7;color:#fff;font-size:.75rem;font-weight:600}}
</style></head><body><div class="wrap">
<h1>Mutanchor report <span class="badge">{prog}</span></h1>
<p class="sub">Generated {when} &middot; {n} mutants</p>
{summary}
<h2>Score by instruction</h2>
<table><thead><tr><th>Instruction</th><th>Killed</th><th>Survived</th><th>Build-failed</th><th>Timeout</th><th>Total</th><th>Score</th></tr></thead>
<tbody>{inst_rows}</tbody></table>
<h2>Mutants</h2>
<table><thead><tr><th>#</th><th>Operator</th><th>Instruction</th><th>Location</th><th>Original</th><th>Mutated</th><th>Exploit / note</th></tr></thead>
<tbody>{rows}</tbody></table>
</div></body></html>"#,
        prog = escape_html(&report.program),
        when = escape_html(&report.generated_at),
        n = report.mutants_total,
        summary = summary,
        inst_rows = inst_rows.join("\n"),
        rows = rows.join("\n"),
    );

    Ok(html)
}

/// Write report files under the given directory:
/// - report.json — the publishable shape the demo site's /dashboard panel
///   renders (matching `frontend/src/lib/report.ts`)
/// - dashboard.json — same shape (kept for the public/ publish step)
/// - report-full.json — the complete detailed report (all mutants, for CI)
/// - report.html — a self-contained HTML scorecard
pub fn write_files(
    report: &Report,
    out_dir: &Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let publish = out_dir.join("report.json");
    let full = out_dir.join("report-full.json");
    let dash = out_dir.join("dashboard.json");
    let html_path = out_dir.join("report.html");
    std::fs::write(&publish, to_publish_json(report)).context("write report.json")?;
    std::fs::write(&full, to_json(report)?).context("write report-full.json")?;
    std::fs::write(&dash, to_publish_json(report)).context("write dashboard.json")?;
    std::fs::write(&html_path, to_html(report)?).context("write report.html")?;
    Ok((publish, html_path))
}

/// The publishable JSON shape the frontend expects. Rendered with camelCase
/// keys to match `frontend/src/lib/report.ts`. A concrete contract so the
/// panel can render the real output of `mutanchor run` with no sample data.
pub fn to_publish_json(report: &Report) -> String {
    let instr: Vec<serde_json::Value> = report
        .per_instruction
        .iter()
        .map(|s| {
            json!({
                "name": s.instruction,
                "killed": s.killed,
                "survived": s.survived,
                "total": s.total,
                "score": s.score(),
            })
        })
        .collect();
    let survivors: Vec<serde_json::Value> = report
        .survivors
        .iter()
        .map(|s| {
            json!({
                "id": s.mutant.id.to_string(),
                "operator": s.mutant.operator.id(),
                "bug_class": s.mutant.operator.bug_class(),
                "file": s.mutant.file,
                "line": s.mutant.line,
                "reason": format!("{} -> {}", s.mutant.original.trim(), s.mutant.mutated.trim()),
                "original": s.mutant.original,
                "mutated": s.mutant.mutated,
                "exploit": s.exploit,
            })
        })
        .collect();
    json!({
        "program": report.program,
        "generatedAt": report.generated_at,
        "totalMutants": report.mutants_total,
        "killed": report.killed,
        "survived": report.survived,
        "buildFailed": report.build_failed,
        "timeout": report.timed_out,
        "mutationScore": report.score(),
        "instructions": instr,
        "survivors": survivors,
    })
    .to_string()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
