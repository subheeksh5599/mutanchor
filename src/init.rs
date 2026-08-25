//! `mutanchor init`: map every instruction to its source file.
//!
//! Reads an Anchor program directory, locates the `#[program]` module's
//! instruction-handler functions, and records each instruction's source file
//! and line range. This drives where mutations are applied and how the report
//! attributes scores per instruction.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use syn::spanned::Spanned;

/// A single instruction handler found in the program.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub name: String,
    /// Relative path (from the project root) of the file defining it.
    pub file: String,
    /// 1-based, inclusive line range of the handler function.
    pub range: std::ops::Range<usize>,
}

/// The result of `init`: which files hold the program's instructions.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub program_dir: PathBuf, // where the Rust program source lives
    pub instructions: Vec<Instruction>,
}

/// Discover the Anchor program directory under `root`.
///
/// Two layouts are supported:
/// - Anchor workspace: `root/Anchor.toml` with program sources under
///   `root/programs/<name>/` (the conventional layout).
/// - Standalone crate: `root/Anchor.toml` (or no Anchor.toml) with the program
///   source directly in `root/src/lib.rs`.
pub fn find_program_dir(root: &Path) -> Result<PathBuf> {
    let root_is_standalone = root.join("src").join("lib.rs").exists() && {
        let ct = root.join("Cargo.toml");
        ct.exists()
            && fs::read_to_string(&ct)
                .map(|t| t.contains("anchor-lang"))
                .unwrap_or(false)
    };

    let anchor_toml = root.join("Anchor.toml");
    if anchor_toml.exists() && !root_is_standalone {
        // Anchor workspace layout: program sources live under programs/.
        let programs_dir = root.join("programs");
        if !programs_dir.is_dir() {
            // Anchor.toml present but no programs/ subdir: fall back to
            // treating the root itself as the program crate.
            return Ok(root.to_path_buf());
        }
        let candidates: Vec<PathBuf> = fs::read_dir(&programs_dir)
            .with_context(|| format!("no programs/ dir under {0}", root.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        if candidates.is_empty() {
            anyhow::bail!("no program directories under {0}/programs", root.display());
        }
        // Prefer a dir whose Cargo.toml declares an anchor-lang dependency.
        for c in &candidates {
            let ct = c.join("Cargo.toml");
            if ct.exists() {
                let text = fs::read_to_string(&ct).unwrap_or_default();
                if text.contains("anchor-lang") {
                    return Ok(c.clone());
                }
            }
        }
        Ok(candidates[0].clone())
    } else {
        // Standalone crate: the root itself is the program dir.
        Ok(root.to_path_buf())
    }
}

/// Scan a program dir and return its instruction manifest. Auto-detects the
/// program directory when the given path is an Anchor workspace root.
pub fn scan(path: &Path) -> Result<Manifest> {
    let program_dir = find_program_dir(path)?;
    let lib = program_dir.join("src").join("lib.rs");
    let text =
        fs::read_to_string(&lib).with_context(|| format!("cannot read {}", lib.display()))?;

    let instructions = find_instructions(&text, &program_dir, &lib)?;

    Ok(Manifest {
        program_dir,
        instructions,
    })
}

/// Locate the `#[program]` module and its handler functions, plus the
/// instruction->file mapping. Handlers may live in lib.rs or inline modules.
fn find_instructions(text: &str, program_dir: &Path, lib: &Path) -> Result<Vec<Instruction>> {
    let _ = (text, program_dir);
    let file =
        syn::parse_file(text).with_context(|| format!("failed to parse {}", lib.display()))?;

    let mut instructions = Vec::new();

    // First pass: find every struct definition (name -> line range) so we can
    // attribute account-constraint mutants to the right handler.
    let mut structs: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    for item in &file.items {
        if let syn::Item::Struct(s) = item {
            let name = s.ident.to_string();
            let start = s.ident.span().start().line.max(1);
            let end = s
                .fields
                .iter()
                .filter_map(|f| f.ty.span().end().line.checked_add(1))
                .max()
                .map(|l| l.max(start + 1))
                .unwrap_or(start + 1);
            structs.push((name, start..end.max(start + 1)));
        }
    }

    // For each handler, record its BODY range and map the Accounts struct it
    // uses (`Context<X>`) to the instruction name. Handler ranges and struct
    // ranges are emitted as SEPARATE, disjoint entries: a mutant line inside a
    // handler body belongs to that handler, a line inside an Accounts struct
    // belongs to the instruction that uses the struct. (Unioning the two into
    // one contiguous range would make ranges overlap — the first match in
    // `attribute` would then win for lines that sit in multiple unions.)
    let mut struct_uses: Vec<(String, String)> = Vec::new(); // (instruction, struct_name)
    let mut pairs: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    for item in &file.items {
        if let syn::Item::Mod(m) = item {
            let is_program = m.attrs.iter().any(|a| a.path().is_ident("program"));
            if !is_program {
                continue;
            }
            let items = if let Some((_, items)) = &m.content {
                items
            } else {
                continue;
            };
            for it in items {
                if let syn::Item::Fn(f) = it {
                    let name = f.sig.ident.to_string();
                    pairs.push((name.clone(), fn_range(f)));
                    if let Some(struct_name) = context_struct(&f.sig) {
                        struct_uses.push((name, struct_name));
                    }
                }
            }
        }
    }

    // Emit each referenced Accounts struct's range under its instruction name.
    let mut struct_ranges: Vec<(String, std::ops::Range<usize>)> = struct_uses
        .iter()
        .filter_map(|(instr, struct_name)| {
            structs
                .iter()
                .find(|(n, _)| n == struct_name)
                .map(|(_, r)| (instr.clone(), r.clone()))
        })
        .collect();

    // Structs not referenced by any handler still get attributed: match them
    // to whichever handler they sit inside.
    for (name, range) in &structs {
        let already_keyed = struct_uses.iter().any(|(_, s)| s == name);
        if already_keyed {
            continue;
        }
        for (n, r) in &pairs {
            if r.contains(&range.start) {
                let union = r.start.min(range.start)..r.end.max(range.end);
                struct_ranges.push((n.clone(), union));
                break;
            }
        }
    }

    pairs.extend(struct_ranges);
    pairs.sort_by_key(|p| p.1.start);

    for (name, range) in pairs {
        instructions.push(Instruction {
            name,
            file: lib
                .strip_prefix(program_dir)
                .unwrap_or(lib)
                .display()
                .to_string(),
            range,
        });
    }

    Ok(instructions)
}

/// Extract the struct name from a `Context<X>` type in a handler signature.
fn context_struct(sig: &syn::Signature) -> Option<String> {
    let mut out: Option<String> = None;
    for input in &sig.inputs {
        if let syn::FnArg::Typed(pat) = input {
            if let syn::Type::Path(tp) = &*pat.ty {
                // NOTE: `Path::is_ident("Context")` would be WRONG here — it
                // requires the path to carry NO generic arguments, so
                // `Context<Deposit>` never matches. Check the first segment.
                let first = tp.path.segments.first().map(|s| s.ident.to_string());
                if first.as_deref() == Some("Context") {
                    if let syn::PathArguments::AngleBracketed(ab) =
                        &tp.path.segments.last()?.arguments
                    {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner))) =
                            ab.args.first()
                        {
                            out = inner.path.get_ident().map(|i| i.to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

/// Approximate the 1-based inclusive line range of a function. With the
/// `span-locations` feature enabled, syn spans from `parse_file` carry real
/// line numbers.
fn fn_range(f: &syn::ItemFn) -> std::ops::Range<usize> {
    let start = f.sig.ident.span().start().line.max(1);
    // The brace span is a DelimSpan: `.open` is `{`, `.close` is `}`. Use the
    // closing brace's line as the end of the function.
    let mut end = start + 1;
    let close_line = f.block.brace_token.span.close().start().line.max(1);
    if close_line > start {
        end = close_line;
    }
    start..end.max(start + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // A minimal Anchor program: one handler + one Accounts struct.
    // Note: the struct's constraint lines come AFTER the handler in the
    // file, so only a working Context<> union attributes them correctly.
    const SRC: &str = r#"
use anchor_lang::prelude::*;

#[program]
pub mod vault {
    use super::*;
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::Zero);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Vault {
    pub balance: u64,
}

#[error_code]
pub enum VaultError {
    #[msg("zero")]
    Zero,
}
"#;

    #[test]
    fn account_struct_constraint_lines_attribute_to_the_using_instruction() {
        let lib = PathBuf::from("/proj/src/lib.rs");
        let instructions = find_instructions(SRC, &PathBuf::from("/proj"), &lib).unwrap();
        // "deposit" gets TWO disjoint ranges: its handler body and the
        // Accounts struct it uses via Context<Deposit>.
        let deposit: Vec<_> = instructions
            .iter()
            .filter(|i| i.name == "deposit")
            .collect();
        assert_eq!(
            deposit.len(),
            2,
            "handler range + struct range both attributed"
        );
        let handler = deposit
            .iter()
            .find(|i| i.range.contains(&8))
            .expect("handler range covers the require! line");
        let struct_range = deposit
            .iter()
            .find(|i| i.range.contains(&17))
            .expect("struct range covers the seeds constraint line");
        // The two ranges must not overlap: a handler-body mutant belongs to
        // the handler, a struct-constraint mutant to the struct, never both.
        assert!(
            handler.range.end <= struct_range.range.start,
            "ranges must be disjoint: {:?} vs {:?}",
            handler.range,
            struct_range.range
        );
        assert_eq!(instructions.len(), 2);
    }
}
