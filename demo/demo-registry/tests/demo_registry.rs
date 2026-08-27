//! Headless LiteSVM test suite for the demo_registry Anchor program.
//!
//! Structure mirrors demo/demo-vault/tests/demo_vault.rs: loads the compiled
//! program .so a single time in-memory (no RPC, no validator) and exercises
//! the four instructions. The program .so is sourced from
//! MUTANCHOR_PROGRAM_SO (set by the parent Mutanchor runner) with a fallback
//! to the local build.

use anchor_lang::InstructionData;
use anchor_litesvm::AnchorLiteSVM;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_sdk::signature::{Keypair, Signer};

use demo_registry::ID as PROGRAM_ID;
use demo_registry::MAX_POINTER;

fn program_elf() -> Vec<u8> {
    let path = std::env::var("MUTANCHOR_PROGRAM_SO").unwrap_or_else(|_| {
        format!("{}/target/deploy/demo_registry.so", env!("CARGO_MANIFEST_DIR"))
    });
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read program .so at {:?} (set MUTANCHOR_PROGRAM_SO or build the \
             program first): {}",
            path, e
        )
    })
}

fn setup() -> anchor_litesvm::AnchorContext {
    let elf = program_elf();
    AnchorLiteSVM::build_with_program(PROGRAM_ID, &elf)
}

fn record_pda(name: &str) -> Pubkey {
    Pubkey::find_program_address(&[b"record", name.as_bytes()], &PROGRAM_ID).0
}

fn claim_ix(record: Pubkey, authority: Pubkey, name: String) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(record, false),
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: demo_registry::instruction::Claim { name }.data(),
    }
}

fn transfer_ix(record: Pubkey, authority: Pubkey, name: String, new_owner: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(record, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: demo_registry::instruction::Transfer {
            _name: name,
            new_owner,
        }
        .data(),
    }
}

fn set_pointer_ix(record: Pubkey, authority: Pubkey, name: String, delta: u64) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(record, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: demo_registry::instruction::SetPointer {
            _name: name,
            delta,
        }
        .data(),
    }
}

fn reclaim_ix(record: Pubkey, authority: Pubkey, name: String) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(record, false),
            AccountMeta::new(authority, true),
        ],
        data: demo_registry::instruction::Reclaim {
            _name: name,
            _tag: 0,
        }
        .data(),
    }
}

fn ensure_claimed(ctx: &mut anchor_litesvm::AnchorContext, authority: &Keypair, name: &str) -> Pubkey {
    let record = record_pda(name);
    ctx.execute_instruction(claim_ix(record, authority.pubkey(), name.to_string()), &[authority])
        .unwrap()
        .assert_success();
    record
}

#[test]
fn claim_creates_record_with_authority_and_zero_pointer() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &authority, "alice");

    let r: demo_registry::NameRecord = ctx.get_account(&record).unwrap();
    assert_eq!(r.owner, authority.pubkey());
    assert_eq!(r.name, "alice");
    assert_eq!(r.pointer, 0);
}

#[test]
fn claim_with_empty_name_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = record_pda("");
    let result = ctx
        .execute_instruction(claim_ix(record, authority.pubkey(), String::new()), &[&authority])
        .unwrap();
    result.assert_failure();
}

// Note: an over-long name would fail *before* the on-chain guard (Solana
// enforces max seed length = 32 at PDA-derivation time), so we can't drive
// the `name.len() <= 32` require! from a real transaction. The guard is
// defensive and untested by design; the length rule is verified by Anchor
// and Solana at a lower layer.

#[test]
fn transfer_by_owner_changes_owner() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &authority, "bob");
    let new_owner = ctx.create_funded_account(0).unwrap();

    ctx.execute_instruction(
        transfer_ix(record, authority.pubkey(), "bob".into(), new_owner.pubkey()),
        &[&authority],
    )
    .unwrap()
    .assert_success();

    let r: demo_registry::NameRecord = ctx.get_account(&record).unwrap();
    assert_eq!(r.owner, new_owner.pubkey());
}

#[test]
fn transfer_by_non_owner_reverts() {
    let mut ctx = setup();
    let owner = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &owner, "carol");
    let attacker = ctx.create_funded_account(10_000_000_000).unwrap();

    let result = ctx
        .execute_instruction(
            transfer_ix(record, attacker.pubkey(), "carol".into(), attacker.pubkey()),
            &[&attacker],
        )
        .unwrap();
    result.assert_failure();

    // Ownership unchanged.
    let r: demo_registry::NameRecord = ctx.get_account(&record).unwrap();
    assert_eq!(r.owner, owner.pubkey());
}

#[test]
fn set_pointer_by_owner_adds_delta() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &authority, "dave");

    ctx.execute_instruction(
        set_pointer_ix(record, authority.pubkey(), "dave".into(), 42),
        &[&authority],
    )
    .unwrap()
    .assert_success();

    let r: demo_registry::NameRecord = ctx.get_account(&record).unwrap();
    assert_eq!(r.pointer, 42);
}

#[test]
fn set_pointer_overflow_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &authority, "eve");

    // First bump pointer to a non-zero value, then try to add u64::MAX so
    // that checked_add overflows.
    ctx.execute_instruction(
        set_pointer_ix(record, authority.pubkey(), "eve".into(), 1),
        &[&authority],
    )
    .unwrap()
    .assert_success();

    let result = ctx
        .execute_instruction(
            set_pointer_ix(record, authority.pubkey(), "eve".into(), u64::MAX),
            &[&authority],
        )
        .unwrap();
    result.assert_failure();

    // State did not advance past the safe increment.
    let r: demo_registry::NameRecord = ctx.get_account(&record).unwrap();
    assert_eq!(r.pointer, 1);
}

#[test]
fn set_pointer_above_max_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &authority, "faye");

    let result = ctx
        .execute_instruction(
            set_pointer_ix(record, authority.pubkey(), "faye".into(), MAX_POINTER + 1),
            &[&authority],
        )
        .unwrap();
    result.assert_failure();
}

#[test]
fn set_pointer_by_non_owner_reverts() {
    let mut ctx = setup();
    let owner = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &owner, "gina");
    let attacker = ctx.create_funded_account(10_000_000_000).unwrap();

    let result = ctx
        .execute_instruction(
            set_pointer_ix(record, attacker.pubkey(), "gina".into(), 1),
            &[&attacker],
        )
        .unwrap();
    result.assert_failure();
}

#[test]
fn reclaim_by_owner_closes_record_and_returns_lamports() {
    let mut ctx = setup();
    let owner = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &owner, "hank");

    let record_lamports_before = ctx.svm.get_account(&record).unwrap().lamports;
    let owner_lamports_before = ctx.svm.get_account(&owner.pubkey()).unwrap().lamports;

    ctx.execute_instruction(reclaim_ix(record, owner.pubkey(), "hank".into()), &[&owner])
        .unwrap()
        .assert_success();

    let closed = ctx.svm.get_account(&record).unwrap();
    assert_eq!(closed.lamports, 0);
    let owner_lamports_after = ctx.svm.get_account(&owner.pubkey()).unwrap().lamports;
    // Owner receives the record's lamports minus LiteSVM's 5000 base fee.
    assert_eq!(
        owner_lamports_after - owner_lamports_before,
        record_lamports_before - 5_000
    );
}

#[test]
fn reclaim_by_non_owner_reverts() {
    let mut ctx = setup();
    let owner = ctx.create_funded_account(10_000_000_000).unwrap();
    let record = ensure_claimed(&mut ctx, &owner, "ivy");
    let attacker = ctx.create_funded_account(10_000_000_000).unwrap();

    let result = ctx
        .execute_instruction(reclaim_ix(record, attacker.pubkey(), "ivy".into()), &[&attacker])
        .unwrap();
    result.assert_failure();

    // Record still exists (has lamports).
    assert!(ctx.svm.get_account(&record).unwrap().lamports > 0);
}
