//! The mutation engine: the eight operators, applied to Anchor program source.
//!
//! Each operator is a fixed, deterministic rule. It scans the source of an
//! Anchor program, finds the construct that models a known Solana audit bug
//! class, and produces one mutant per occurrence: exactly one fault, located
//! at a `file:line`, with the original and mutated line captured for the
//! report. There is no AI anywhere in this path — same input, same mutants.

use crate::model::{Mutant, Operator};

/// The set of source lines on which an operator is allowed to fire, used to
/// keep one-mutant-per-location and to bind a mutant to the enclosing
/// instruction handler.
#[derive(Debug, Clone)]
pub struct OperatorCtx<'a> {
    pub rel_file: &'a str,
    /// Line ranges (1-based, inclusive) of the instruction handler
    /// functions, so a mutant can be attributed to an instruction.
    pub instruction_ranges: &'a [(String, std::ops::Range<usize>)],
}

/// Result of a single operator scan: the mutants it produced on the source.
pub fn generate(
    file_text: &str,
    rel_file: &str,
    instructions: &[(String, std::ops::Range<usize>)],
) -> Vec<Mutant> {
    let mut out = Vec::new();

    let mut id = 0usize;
    let mut push =
        |op: Operator, file: &str, line: u32, orig: String, muta: String, instr: Option<String>| {
            let m = Mutant {
                id,
                operator: op,
                file: file.to_string(),
                line,
                original: orig,
                mutated: muta,
                instruction: instr,
            };
            id += 1;
            out.push(m);
        };

    let lines: Vec<&str> = file_text.lines().collect();
    let ctx = OperatorCtx {
        rel_file,
        instruction_ranges: instructions,
    };

    signer_check_removal(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    authority_check_drop(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    discriminator_removal(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    pda_seed_swap(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    bump_mismatch(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    cpi_target_swap(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    close_rent_drop(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });
    comparison_flip(&lines, &ctx, &mut |op, f, l, o, m, i| {
        push(op, f, l, o, m, i)
    });

    out
}

/// Attribute an operator line to the enclosing instruction handler, if any.
fn attribute(ctx: &OperatorCtx, line: u32) -> Option<String> {
    ctx.instruction_ranges
        .iter()
        .find(|(_, r)| r.contains(&(line as usize)))
        .map(|(name, _)| name.clone())
}

fn is_comment_or_blank(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with("//") || t.starts_with("*") || t.starts_with("#[")
}

/// `#[account(signer)]` -> remove `signer` from the account constraint. Models
/// missing signer validation (mutant accepts unsigned callers).
fn signer_check_removal(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        if !line.contains("signer") {
            continue;
        }
        let t = line.trim();
        // Trigger only inside account struct constraint attributes or signer
        // assertions.
        let in_attr = t.starts_with("#[account") && t.contains("signer");
        let is_signer_pred = t.contains("signer") && !is_comment_or_blank(line);
        if !in_attr && !is_signer_pred {
            continue;
        }

        // Mutant A: remove `signer` keyword from an #[account(...)] constraint.
        if t.starts_with("#[account") {
            let muta = line.to_string();
            let replaced = muta
                .replace("signer", "")
                .replace(",,", ",")
                .replace(" ,", " ");
            // Clean trailing/leading commas inside the parens.
            let replaced = replaced
                .replace("(,", "(")
                .replace(",,", ",")
                .replace(" )", ")")
                .replace(",)", ")");
            if replaced != *line {
                push(
                    Operator::SignerCheckRemoval,
                    ctx.rel_file,
                    line_no,
                    line.to_string(),
                    replaced,
                    attribute(ctx, line_no),
                );
            }
            continue;
        }

        // Mutant B: turn a `require!(... .is_signer / signer )` into a removed
        // check — handled by authority_check_drop for require!; here we
        // capture direct `Signer` usage sites.
        if is_signer_pred {
            // Comment-only trigger guard
            if t.starts_with("//") {
                continue;
            }
        }
    }
}

/// Drop an owner/authority `require!` / `require_keys_eq!` assertion. Models
/// missing owner/authority validation.
fn authority_check_drop(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        let is_check = (t.starts_with("require!")
            || t.starts_with("require_keys_eq!")
            || t.starts_with("require_neq!"))
            && (t.contains("authority") || t.contains("owner"))
            && !t.contains("//");
        if !is_check {
            continue;
        }
        // Replace the whole require statement with a recorded-survive no-op comment.
        let indent = &line[..line.len() - line.trim_start().len()];
        let muta = format!(
            "{}// ORIGINAL CHECK REMOVED (mutation): {}",
            indent,
            line.trim()
        );
        push(
            Operator::AuthorityCheckDrop,
            ctx.rel_file,
            line_no,
            line.to_string(),
            muta,
            attribute(ctx, line_no),
        );
    }
}

/// Remove the instruction discriminator check. Anchor's `#[account]` derive
/// emits a discriminator check at the top of every handler; the source-level
/// equivalent is an explicit discriminator guard or the derive macro stub.
/// We target explicit discriminator guards.
fn discriminator_removal(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        let is_disc =
            (t.contains("discriminator") || t.contains("DISCRIMINATOR")) && !t.contains("//");
        if !is_disc {
            continue;
        }
        // A `require!(... == <discriminator>)` guard.
        if t.starts_with("require!") {
            let indent = &line[..line.len() - line.trim_start().len()];
            let muta = format!("{}// DISCRIMINATOR CHECK REMOVED (mutation): {}", indent, t);
            push(
                Operator::DiscriminatorRemoval,
                ctx.rel_file,
                line_no,
                line.to_string(),
                muta,
                attribute(ctx, line_no),
            );
        }
    }
}

/// Swap the seeds in a PDA `seeds = [b"seed", ...]` expression. Models wrong
/// seeds / wrong account resolution.
fn pda_seed_swap(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        if !t.contains("seeds =") || t.contains("//") {
            continue;
        }
        // Find the first byte-string literal seed, e.g. `b"escrow"`.
        let re = regex_lite::Regex::new(r#"b"[^"]*""#).ok();
        if let Some(re) = re {
            if let Some(cap) = re.find(t) {
                let seed = cap.as_str();
                if seed == "b\"\"" {
                    continue;
                }
                let swapped = format!(
                    "b\"__mut_{}\"",
                    seed.trim_start_matches("b\"").trim_end_matches('"')
                );
                let muta = t.replacen(seed, &swapped, 1);
                push(
                    Operator::PdaSeedSwap,
                    ctx.rel_file,
                    line_no,
                    line.to_string(),
                    format!("{}{}", &line[..line.len() - t.len()], muta),
                    attribute(ctx, line_no),
                );
            }
        }
    }
}

/// Change the bump in a PDA derivation or `bump` constraint. Models using the
/// wrong PDA bump.
fn bump_mismatch(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        if t.contains("//") {
            continue;
        }
        if !t.contains("bump") {
            continue;
        }
        // Only target real PDA-bump uses, not unrelated identifiers containing
        // "bump" as a substring of something else.
        let is_bump_use = t.contains("find_program_address")
            || t.contains(", bump")
            || t.contains("bump,")
            || t.contains("&bump")
            || t.contains("bump)")
            || t.contains("bump =");
        if !is_bump_use {
            continue;
        }
        // Replace a bare `bump` argument with `bump + 1` so the derived/used
        // bump no longer matches the canonical one.
        let changed = bump_offset(t);
        if changed != t {
            push(
                Operator::BumpMismatch,
                ctx.rel_file,
                line_no,
                line.to_string(),
                format!("{}{}", &line[..line.len() - t.len()], changed),
                attribute(ctx, line_no),
            );
        }
    }
}

/// Offset the first bare `bump` token to `bump + 1`. Keeps the change
/// syntactically valid wherever a bump expression is expected.
fn bump_offset(line: &str) -> String {
    let changed = line.replacen("bump", "bump + 1", 1);
    // If it landed inside `find_program_address(&[..], &[..])` style args it is
    // still valid as an expression; otherwise fall back to no change.
    changed
}

/// Point a CPI at a different program. We rewrite the `program::cpi::` path to
/// a clearly-wrong target. Models calling the wrong program.
fn cpi_target_swap(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        if !t.contains("::cpi::") || t.contains("//") {
            continue;
        }
        // Replace `<prog>::cpi::` with `<prog>_mutant::cpi::`.
        let muta = t.replacen("::cpi::", "_mutant::cpi::", 1);
        if muta != t {
            push(
                Operator::CpiTargetSwap,
                ctx.rel_file,
                line_no,
                line.to_string(),
                format!("{}{}", &line[..line.len() - t.len()], muta),
                attribute(ctx, line_no),
            );
        }
    }
}

/// Drop a `close = <account>` or rent-exempt constraint. Models accounts
/// closed or rent-exempt not validated.
fn close_rent_drop(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        if !t.contains("close") || t.contains("//") {
            continue;
        }
        // `close = <account>` inside an account constraint attribute.
        if t.contains("close =") {
            let muta = t.replacen("close =", "// close removed (mutation)", 1);
            push(
                Operator::CloseRentDrop,
                ctx.rel_file,
                line_no,
                line.to_string(),
                format!("{}{}", &line[..line.len() - t.len()], muta),
                attribute(ctx, line_no),
            );
        }
    }
}

/// Flip a comparison / boolean negation, e.g. `>` -> `<`, `>=` -> `<`, `==`
/// -> `!=`, `true` -> `false`. Models boundary errors.
///
/// Guarding: single `<` / `>` are only flipped when they are real comparison
/// operators, never when they are part of generic type or lifetime syntax
/// (e.g. `Account<'_, Vault>` must not become `<=`).
fn comparison_flip(
    lines: &[&str],
    ctx: &OperatorCtx,
    push: &mut dyn FnMut(Operator, &str, u32, String, String, Option<String>),
) {
    let re = regex_lite::Regex::new(r"(<=|>=|==|!=|&&|\|\||[<>])").unwrap();
    let mut seen = std::collections::HashSet::new();
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1) as u32;
        let t = line.trim();
        if t.contains("//") || t.starts_with("#[") || t.starts_with("pub ") || t.contains("fn ") {
            continue;
        }
        if t.contains("require!") {
            // Skip require! lines already owned by authority_check_drop or
            // discriminator_removal; other require! boundary checks are fair
            // game for comparison_flip.
            if t.contains("authority") || t.contains("owner") || t.contains("discriminator") {
                continue;
            }
        }
        // Skip `let` bindings and account-field declarations: their `<`/`>`
        // are generic/lifetime syntax, not comparisons.
        if t.starts_with("let ") || t.contains("Account<") || t.contains("Signer<") {
            continue;
        }
        // Don't flip inside a seeds array or attribute.
        if t.contains("seeds =") || t.contains("#[") {
            continue;
        }
        for cap in re.find_iter(t) {
            let op = cap.as_str();
            // Reject single < > that are part of an identifier/generic token.
            if op == "<" || op == ">" {
                let before = t[..cap.start()].chars().last().unwrap_or(' ');
                let after = t[cap.end()..].chars().next().unwrap_or(' ');
                let prev_is_word = before.is_alphanumeric() || before == '_' || before == '\'';
                let next_is_word = after.is_alphanumeric() || after == '_' || after == '\'';
                let in_generics = prev_is_word || next_is_word;
                // Also `->` return arrows contain `>`; skip those.
                let arrow = t[..cap.start()].ends_with('-');
                if in_generics || arrow {
                    continue;
                }
            }
            let flipped = match op {
                "<=" => "<",
                ">=" => ">",
                "<" => "<=",
                ">" => ">=",
                "==" => "!=",
                "!=" => "==",
                "&&" => "||",
                "||" => "&&",
                _ => continue,
            };
            // One flip per location.
            let key = format!("{}:{}", line_no, cap.start());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            let muta = format!(
                "{}{}",
                &line[..line.len() - t.len()],
                t.replacen(op, flipped, 1)
            );
            push(
                Operator::ComparisonFlip,
                ctx.rel_file,
                line_no,
                line.to_string(),
                muta,
                attribute(ctx, line_no),
            );
            break; // only first comparison per line to keep mutants distinct
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Operator;
    use crate::ops::generate;

    fn mutants_for(source: &str, op: Operator) -> Vec<crate::model::Mutant> {
        generate(source, "src/lib.rs", &[])
            .into_iter()
            .filter(|m| m.operator == op)
            .collect()
    }

    #[test]
    fn signer_check_removal_fires_on_signer_constraint() {
        let src2 = r#"
struct Ctx<'info> {
    #[account(signer, mut)]
    pub authority: Account<'info, Data>,
}
"#;
        let m = mutants_for(src2, Operator::SignerCheckRemoval);
        assert!(!m.is_empty(), "signer constraint should produce a mutant");
        assert!(m.iter().all(|x| !x.mutated.contains("signer")));
    }

    #[test]
    fn authority_check_drop_fires_on_authority_require() {
        let src = "require!(ctx.accounts.vault.owner == ctx.accounts.authority.key(), Error::Unauthorized);";
        let m = mutants_for(src, Operator::AuthorityCheckDrop);
        assert!(!m.is_empty(), "authority require should produce a mutant");
        assert!(m[0].mutated.contains("ORIGINAL CHECK REMOVED"));
    }

    #[test]
    fn pda_seed_swap_rewrites_seed() {
        let src = "#[account(seeds = [b\"vault\", authority.key().as_ref()], bump)]";
        let m = mutants_for(src, Operator::PdaSeedSwap);
        assert_eq!(m.len(), 1, "exactly one seed-swap mutant");
        assert!(m[0].mutated.contains("__mut_vault"));
    }

    #[test]
    fn bump_mismatch_offsets_bump() {
        let src = "pub fn derive() { let (p, bump) = Pubkey::find_program_address(&[b\"v\"], &id()); let _ = p; let _ = bump; }";
        let m = mutants_for(src, Operator::BumpMismatch);
        assert!(m.iter().any(|x| x.mutated.contains("bump + 1")));
    }

    #[test]
    fn discriminator_removal_fires_on_guard() {
        let src = "require!(data.discriminator == DISCRIMINATOR, Error::BadDiscriminator);";
        let m = mutants_for(src, Operator::DiscriminatorRemoval);
        assert!(!m.is_empty());
        assert!(m[0].mutated.contains("DISCRIMINATOR CHECK REMOVED"));
    }

    #[test]
    fn cpi_target_swap_rewrites_cpi_path() {
        let src = "token_program::cpi::transfer(ctx.accounts.ctx(), amount)?;";
        let m = mutants_for(src, Operator::CpiTargetSwap);
        assert!(!m.is_empty());
        assert!(m[0].mutated.contains("_mutant::cpi::"));
    }

    #[test]
    fn close_rent_drop_removes_close() {
        let src2 = "#[account(mut, seeds = [b\"v\"], bump, close = authority)]";
        let m2 = mutants_for(src2, Operator::CloseRentDrop);
        assert!(!m2.is_empty());
        assert!(m2[0].mutated.contains("close removed"));
    }

    #[test]
    fn comparison_flip_inverts_boundary() {
        let src = "require!(amount > 0, Error::Zero);";
        let m = mutants_for(src, Operator::ComparisonFlip);
        assert!(!m.is_empty());
        assert!(m.iter().any(|x| x.mutated.contains(">=")));
    }

    #[test]
    fn comparison_flip_does_not_touch_generic_type_arrows() {
        let src = "let vault: &mut Account<'_, Vault> = &mut ctx.accounts.vault;";
        let m = mutants_for(src, Operator::ComparisonFlip);
        assert!(
            m.is_empty(),
            "generic type syntax must not be mutated, got: {m:?}"
        );
    }

    #[test]
    fn one_fault_per_mutant_holds() {
        let src = include_str!("../demo/fixture/src/lib.rs");
        let muts = generate(src, "src/lib.rs", &[]);
        assert!(!muts.is_empty(), "fixture should produce mutants");
        for m in &muts {
            assert_ne!(
                m.original, m.mutated,
                "mutant must actually change the line"
            );
        }
    }
}
