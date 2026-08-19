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
    let root_is_standalone = root.join("src").join("lib.rs").exists()
        && {
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

    // For each handler, find the Accounts struct it uses (via `Context<X>`)
    // and record both the handler range and the struct range so mutants in
    // account constraints attribute to the instruction.
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
                    let mut range = fn_range(f);
                    // Find `Context<X>` in the signature.
                    if let Some(struct_name) = context_struct(&f.sig) {
                        if let Some((_, r)) = structs.iter().find(|(n, _)| n == &struct_name) {
                            range = range.start.min(r.start)..range.end.max(r.end);
                        }
                    }
                    pairs.push((name, range));
                }
            }
        }
    }

    // Accounts structs referenced by a handler get their constraint lines
    // attributed to that instruction. Structs not yet covered are matched to
    // whichever handler they sit inside.
    let mut unions: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    for (name, range) in &structs {
        let already_keyed = pairs
            .iter()
            .any(|(n, r)| n == name || r.start == range.start);
        if already_keyed {
            continue;
        }
        // Find a handler whose range contains the struct, then union them.
        for (n, r) in &pairs {
            if r.contains(&range.start) {
                let union = r.start.min(range.start)..r.end.max(range.end);
                unions.push((n.clone(), union));
                break;
            }
        }
    }
    for (n, u) in unions {
        if let Some(e) = pairs.iter_mut().find(|(nn, _)| *nn == n) {
            e.1 = u;
        }
    }

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
                if tp.path.is_ident("Context") {
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
