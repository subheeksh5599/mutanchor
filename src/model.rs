//! Shared data model for Mutanchor.
//!
//! Every mutant is one injected fault at one `file:line`, produced by one
//! operator. A run classifies each mutant into one of four verdicts. The report
//! aggregates verdicts into per-instruction and overall mutation scores.

use serde::{Deserialize, Serialize};

/// The ten operators. Each models a bug class from the Solana
/// audit-findings corpus. The discriminant doubles as a stable machine id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operator {
    /// Remove a `signer` constraint / signer assertion.
    SignerCheckRemoval,
    /// Swap the seeds in a PDA `seeds = [...]` expression.
    PdaSeedSwap,
    /// Change the bump used to derive a PDA.
    BumpMismatch,
    /// Drop an owner/authority `require!` check.
    AuthorityCheckDrop,
    /// Remove an Anchor discriminator check.
    DiscriminatorRemoval,
    /// Point a CPI at a different program.
    CpiTargetSwap,
    /// Drop a `close` / rent-exempt constraint.
    CloseRentDrop,
    /// Flip a comparison/boolean boundary.
    ComparisonFlip,
    /// Replace a `checked_add` / `checked_sub` / `checked_mul` with the
    /// unchecked (`+` / `-` / `*`) form. Models silent arithmetic overflow.
    UncheckedMath,
    /// Drop a `realloc` size guard or size-check on account resize. Models
    /// unchecked account resizing / re-initialization.
    ReallocCheckDrop,
}

impl Operator {
    /// All operators, in README table order.
    pub const ALL: [Operator; 10] = [
        Operator::SignerCheckRemoval,
        Operator::PdaSeedSwap,
        Operator::BumpMismatch,
        Operator::AuthorityCheckDrop,
        Operator::DiscriminatorRemoval,
        Operator::CpiTargetSwap,
        Operator::CloseRentDrop,
        Operator::ComparisonFlip,
        Operator::UncheckedMath,
        Operator::ReallocCheckDrop,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Operator::SignerCheckRemoval => "signer_check_removal",
            Operator::PdaSeedSwap => "pda_seed_swap",
            Operator::BumpMismatch => "bump_mismatch",
            Operator::AuthorityCheckDrop => "authority_check_drop",
            Operator::DiscriminatorRemoval => "discriminator_removal",
            Operator::CpiTargetSwap => "cpi_target_swap",
            Operator::CloseRentDrop => "close_rent_drop",
            Operator::ComparisonFlip => "comparison_flip",
            Operator::UncheckedMath => "unchecked_math",
            Operator::ReallocCheckDrop => "realloc_check_drop",
        }
    }

    /// The Solana audit bug class this operator models.
    pub fn bug_class(self) -> &'static str {
        match self {
            Operator::SignerCheckRemoval => "missing signer validation",
            Operator::PdaSeedSwap => "wrong seeds / wrong account resolution",
            Operator::BumpMismatch => "wrong PDA bump",
            Operator::AuthorityCheckDrop => "missing owner/authority validation",
            Operator::DiscriminatorRemoval => "accepting wrong instruction discriminators",
            Operator::CpiTargetSwap => "calling the wrong program",
            Operator::CloseRentDrop => "accounts closed or rent-exempt not validated",
            Operator::ComparisonFlip => "boundary errors (off-by-one, wrong operator)",
            Operator::UncheckedMath => "silent arithmetic overflow (checked_* dropped)",
            Operator::ReallocCheckDrop => "unchecked account resize / re-init",
        }
    }
}

/// A single injected fault: one operator applied at one location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutant {
    /// Stable index within the run.
    pub id: usize,
    /// Operator that produced this mutant.
    pub operator: Operator,
    /// Source file the change was made in (path relative to program root).
    pub file: String,
    /// 1-based line of the change.
    pub line: u32,
    /// The original source line (for the report).
    pub original: String,
    /// The mutated source line (for the report).
    pub mutated: String,
    /// Which instruction handler this change falls under (best-effort; may be
    /// empty if the tool could not attribute it).
    pub instruction: Option<String>,
}

/// How the test suite fared against one mutant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// At least one test failed -> the change was caught.
    Killed,
    /// All tests passed -> the change survived. A real bug your tests miss.
    Survived,
    /// The mutant source did not compile.
    BuildFailed,
    /// The test run did not finish inside the timeout.
    TimedOut,
}

impl Verdict {
    pub fn id(self) -> &'static str {
        match self {
            Verdict::Killed => "killed",
            Verdict::Survived => "survived",
            Verdict::BuildFailed => "build-failed",
            Verdict::TimedOut => "timeout",
        }
    }
}

/// Outcome of executing one mutant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutantResult {
    pub mutant: Mutant,
    pub verdict: Verdict,
    /// For killed mutants: number of failing tests and the first failure name.
    pub failing_tests: usize,
    pub first_failure: Option<String>,
    /// For survivors: an attacker exploit sketch.
    pub exploit: Option<String>,
    pub build_ms: u128,
    pub run_ms: u128,
}

impl Default for MutantResult {
    fn default() -> Self {
        MutantResult {
            mutant: Mutant {
                id: 0,
                operator: Operator::SignerCheckRemoval,
                file: String::new(),
                line: 0,
                original: String::new(),
                mutated: String::new(),
                instruction: None,
            },
            verdict: Verdict::Killed,
            failing_tests: 0,
            first_failure: None,
            exploit: None,
            build_ms: 0,
            run_ms: 0,
        }
    }
}

/// Per-instruction aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionScore {
    pub instruction: String,
    pub killed: usize,
    pub survived: usize,
    pub build_failed: usize,
    pub timed_out: usize,
    pub total: usize,
}

impl InstructionScore {
    /// Ratio of non-surviving verdicts to all MEANINGFUL verdicts. Survived
    /// drags it down; build-failed counts as a non-escape (the mutation
    /// provably broke the build). Timeouts are INCONCLUSIVE — we learned
    /// nothing — so they are excluded from the denominator entirely instead
    /// of being counted as escapes or non-escapes.
    pub fn score(&self) -> f64 {
        let denom = self.killed + self.survived + self.build_failed;
        if denom == 0 {
            0.0
        } else {
            (self.killed + self.build_failed) as f64 / denom as f64
        }
    }
}

/// The complete result of a `mutanchor run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub program: String,
    pub generated_at: String,
    pub mutants_total: usize,
    pub killed: usize,
    pub survived: usize,
    pub build_failed: usize,
    pub timed_out: usize,
    pub dropped_equivalent: usize,
    pub per_instruction: Vec<InstructionScore>,
    pub survivors: Vec<MutantResult>,
    pub mutants: Vec<MutantResult>,
}

impl Report {
    /// Overall mutation score: fraction of MEANINGFUL verdicts that did NOT
    /// survive (i.e. the test suite actually proved something about them).
    /// Same semantics as `InstructionScore::score`: build-failed counts as
    /// a non-escape, timeouts are inconclusive and excluded.
    pub fn score(&self) -> f64 {
        let denom = self.killed + self.survived + self.build_failed;
        if denom == 0 {
            0.0
        } else {
            (self.killed + self.build_failed) as f64 / denom as f64
        }
    }

    pub fn is_empty(&self) -> bool {
        self.mutants_total == 0
    }
}
