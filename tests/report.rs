//! Report rendering tests: the publishable `report.json` must match the exact
//! camelCase contract the frontend `/dashboard` panel consumes
//! (`frontend/src/lib/report.ts`), and `report.html` must be self-contained
//! valid HTML.

use mutanchor::model::{Mutant, MutantResult, Operator, Report, Verdict};
use mutanchor::report::{to_html, to_publish_json};

fn sample_report() -> Report {
    let mk = |id: usize, op: Operator, verdict: Verdict, line: u32, orig: &str, mutd: &str| {
        MutantResult {
            mutant: Mutant {
                id,
                operator: op,
                file: "src/lib.rs".to_string(),
                line,
                original: orig.to_string(),
                mutated: mutd.to_string(),
                instruction: Some("deposit".to_string()),
            },
            verdict,
            failing_tests: if matches!(verdict, Verdict::Killed) {
                1
            } else {
                0
            },
            first_failure: if matches!(verdict, Verdict::Killed) {
                Some("boundary".to_string())
            } else {
                None
            },
            exploit: if matches!(verdict, Verdict::Survived) {
                Some("attacker can overdraw".to_string())
            } else {
                None
            },
            build_ms: 5,
            run_ms: 2,
        }
    };

    let killed = mk(
        0,
        Operator::ComparisonFlip,
        Verdict::Killed,
        31,
        "require!(amount > 0)",
        "require!(amount >= 0)",
    );
    let survived = mk(
        1,
        Operator::ComparisonFlip,
        Verdict::Survived,
        43,
        "require!(a <= b)",
        "require!(a < b)",
    );

    // Per-instruction aggregate mirrors what the engine builds: one deposit
    // instruction with 1 killed + 1 survived.
    use mutanchor::model::InstructionScore;
    Report {
        program: "demo_vault".to_string(),
        generated_at: "2026-08-19T00:00:00Z".to_string(),
        mutants_total: 2,
        killed: 1,
        survived: 1,
        build_failed: 0,
        timed_out: 0,
        dropped_equivalent: 0,
        per_instruction: vec![InstructionScore {
            instruction: "deposit".to_string(),
            killed: 1,
            survived: 1,
            build_failed: 0,
            timed_out: 0,
            total: 2,
        }],
        survivors: vec![survived.clone()],
        mutants: vec![killed, survived],
    }
}

#[test]
fn publishable_json_matches_frontend_contract() {
    let report = sample_report();
    let json: serde_json::Value = serde_json::from_str(&to_publish_json(&report)).unwrap();

    // Top-level keys the frontend's MutationReport interface expects.
    for key in [
        "program",
        "generatedAt",
        "totalMutants",
        "killed",
        "survived",
        "buildFailed",
        "timeout",
        "mutationScore",
        "instructions",
        "survivors",
    ] {
        assert!(json.get(key).is_some(), "missing top-level key: {key}");
    }
    assert_eq!(json["totalMutants"], 2);
    assert_eq!(json["killed"], 1);
    assert_eq!(json["survived"], 1);
    // Score is a 0..1 fraction (frontend multiplies by 100 for display).
    assert_eq!(json["mutationScore"], 0.5);

    // Per-instruction shape.
    let ins = &json["instructions"][0];
    for key in ["name", "killed", "survived", "total", "score"] {
        assert!(ins.get(key).is_some(), "missing instruction key: {key}");
    }
    assert_eq!(ins["name"], "deposit");
    assert_eq!(ins["score"], 0.5);

    // Survivor shape (frontend uses id/operator/file/line/reason/exploit).
    let s = &json["survivors"][0];
    for key in [
        "id", "operator", "file", "line", "reason", "exploit", "original", "mutated",
    ] {
        assert!(s.get(key).is_some(), "missing survivor key: {key}");
    }
    assert_eq!(s["operator"], "comparison_flip");
    assert_eq!(s["line"], 43);
    assert!(
        s["reason"].as_str().unwrap().contains("->"),
        "reason should capture original -> mutated"
    );
}

#[test]
fn report_score_is_fraction_and_report_html_is_valid() {
    let report = sample_report();
    assert_eq!(report.score(), 0.5);

    let html = to_html(&report).unwrap();
    assert!(html.contains("<!doctype html>"));
    assert!(html.contains("Mutation score"));
    assert!(html.contains("demo_vault"));
    // Self-contained: no external script/style references (no network deps).
    assert!(!html.contains("src=\"http"));
    assert!(!html.contains("href=\"http"));
}

#[test]
fn empty_report_renders_empty_state_not_fabricated_data() {
    let empty = Report {
        program: "demo_vault".to_string(),
        generated_at: "2026-08-19T00:00:00Z".to_string(),
        mutants_total: 0,
        killed: 0,
        survived: 0,
        build_failed: 0,
        timed_out: 0,
        dropped_equivalent: 0,
        per_instruction: Vec::new(),
        survivors: Vec::new(),
        mutants: Vec::new(),
    };
    let html = to_html(&empty).unwrap();
    // The panel must say it's empty, never show fabricated numbers.
    assert!(html.contains("No run yet"));
    // Empty score render should not crash.
    let _ = to_publish_json(&empty);
}

#[test]
fn all_timeout_report_scores_zero_not_vacuous_one() {
    // A run where every mutant timed out learned nothing: the score must be
    // 0.0 (inconclusive), never a vacuous 100%. This pins the honesty rule.
    let report = Report {
        program: "demo_vault".to_string(),
        generated_at: "2026-08-19T00:00:00Z".to_string(),
        mutants_total: 13,
        killed: 0,
        survived: 0,
        build_failed: 0,
        timed_out: 13,
        dropped_equivalent: 0,
        per_instruction: vec![],
        survivors: vec![],
        mutants: vec![],
    };
    assert_eq!(report.score(), 0.0);
}
