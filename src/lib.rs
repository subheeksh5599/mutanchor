//! Mutanchor — mutation testing for Solana Anchor programs.
//!
//! Library surface: the deterministic mutation engine, the LiteSVM runner,
//! and the report renderer are exposed here so they can be unit-tested and
//! reused by the CLI binary.

// The mutation-engine push callback has one fixed, intentional signature
// (operator + location + original + mutated + instruction). It is verbose by
// design; the alias keeps call sites identical across all eight operators.
#![allow(clippy::type_complexity)]

pub mod engine;
pub mod init;
pub mod model;
pub mod ops;
pub mod report;
pub mod runner;
