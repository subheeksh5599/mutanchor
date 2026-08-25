//! Headless LiteSVM test suite for the demo_vault Anchor program.
//!
//! Loads the compiled program .so a single time in-memory (no RPC, no
//! validator) and exercises the deposit / withdraw / close instructions.
//!
//! The program .so is sourced from the `MUTANCHOR_PROGRAM_SO` environment
//! variable (set by the parent Mutanchor runner to point at each mutant's
//! freshly built binary). When unset we fall back to the local build at
//! `target/deploy/demo_vault.so` relative to this crate.

use anchor_lang::InstructionData;
use anchor_litesvm::AnchorLiteSVM;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_sdk::signature::{Keypair, Signer};

use demo_vault::ID as PROGRAM_ID;

/// Locate and read the compiled program binary.
fn program_elf() -> Vec<u8> {
    let path = std::env::var("MUTANCHOR_PROGRAM_SO").unwrap_or_else(|_| {
        // Compile-time crate dir: /home/arch/mutanchor/demo/demo-vault
        format!("{}/target/deploy/demo_vault.so", env!("CARGO_MANIFEST_DIR"))
    });
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read program .so at {:?} (set MUTANCHOR_PROGRAM_SO or build the \
             program first): {}",
            path, e
        )
    })
}

/// Build a fresh in-memory chain with demo_vault deployed.
fn setup() -> anchor_litesvm::AnchorContext {
    let elf = program_elf();
    AnchorLiteSVM::build_with_program(PROGRAM_ID, &elf)
}

/// The vault PDA for a given authority.
fn vault_pda(authority: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"vault", authority.as_ref()], &PROGRAM_ID).0
}

fn deposit_ix(vault: Pubkey, authority: Pubkey, amount: u64) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(vault, false),    // writable vault
            AccountMeta::new(authority, true), // signer
        ],
        data: demo_vault::instruction::Deposit { amount }.data(),
    }
}

fn create_ix(vault: Pubkey, authority: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(vault, false),              // init, writable
            AccountMeta::new(authority, true),           // signer + payer
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ],
        data: demo_vault::instruction::Create {}.data(),
    }
}

/// Ensure the vault exists by running `create` (idempotent per authority).
fn ensure_vault(ctx: &mut anchor_litesvm::AnchorContext, authority: &Keypair) -> Pubkey {
    let vault = vault_pda(&authority.pubkey());
    ctx.execute_instruction(create_ix(vault, authority.pubkey()), &[authority])
        .unwrap()
        .assert_success();
    vault
}

fn withdraw_ix(vault: Pubkey, authority: Pubkey, amount: u64) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(vault, false),    // writable vault
            AccountMeta::new(authority, true), // signer
        ],
        data: demo_vault::instruction::Withdraw { amount }.data(),
    }
}

fn pay_ix(authority: Pubkey, recipient: Pubkey, amount: u64) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(authority, true),              // signer + from
            AccountMeta::new(recipient, false),             // to
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ],
        data: demo_vault::instruction::Pay { amount }.data(),
    }
}

fn close_ix(vault: Pubkey, authority: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(vault, false),    // writable, close -> authority
            AccountMeta::new(authority, true), // signer, receives lamports
        ],
        data: demo_vault::instruction::Close { _tag: 0 }.data(),
    }
}

#[test]
fn deposit_creates_vault_and_increments_balance() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &authority);

    // Vault exists after create and starts at zero.
    assert!(ctx.account_exists(&vault));

    ctx.execute_instruction(deposit_ix(vault, authority.pubkey(), 1_000), &[&authority])
        .unwrap()
        .assert_success();

    // The vault records the deposit on its `balance` field.
    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.owner, authority.pubkey());
    assert_eq!(v.balance, 1_000);
}

#[test]
fn withdraw_within_balance_succeeds_and_decreases_balance() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &authority);

    ctx.execute_instruction(deposit_ix(vault, authority.pubkey(), 1_000), &[&authority])
        .unwrap()
        .assert_success();

    ctx.execute_instruction(withdraw_ix(vault, authority.pubkey(), 400), &[&authority])
        .unwrap()
        .assert_success();

    // Withdraw reduces the bookkeeping balance.
    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.balance, 600);
}

#[test]
fn pay_cpi_transfers_lamports_to_recipient() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let recipient = ctx.create_funded_account(0).unwrap();

    let before = ctx.svm.get_account(&recipient.pubkey()).unwrap().lamports;

    ctx.execute_instruction(pay_ix(authority.pubkey(), recipient.pubkey(), 123_456), &[&authority])
        .unwrap()
        .assert_success();

    // The system-program CPI actually moved lamports.
    let after = ctx.svm.get_account(&recipient.pubkey()).unwrap().lamports;
    assert_eq!(after - before, 123_456);
}

#[test]
fn withdraw_over_balance_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &authority);

    ctx.execute_instruction(deposit_ix(vault, authority.pubkey(), 1_000), &[&authority])
        .unwrap()
        .assert_success();

    // Attempt to withdraw more than the recorded balance -> `Insufficient`.
    // The revert itself is the signal: a flipped (mutated) `<=` boundary
    // check would let this transaction succeed.
    let result = ctx
        .execute_instruction(withdraw_ix(vault, authority.pubkey(), 5_000), &[&authority])
        .unwrap();
    result.assert_failure();

    // State is unchanged after the revert.
    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.balance, 1_000);
}

#[test]
fn unauthorized_close_reverts() {
    let mut ctx = setup();
    let owner = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &owner);

    ctx.execute_instruction(deposit_ix(vault, owner.pubkey(), 1_000), &[&owner])
        .unwrap()
        .assert_success();

    // An attacker who is NOT the vault owner tries to close the owner's vault.
    let attacker = ctx.create_funded_account(10_000_000_000).unwrap();

    let result = ctx
        .execute_instruction(close_ix(vault, attacker.pubkey()), &[&attacker])
        .unwrap();
    result.assert_failure();

    // The vault still exists and was not touched.
    assert!(ctx.account_exists(&vault));
    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.balance, 1_000);
}

#[test]
fn deposit_zero_amount_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &authority);

    // Zero-amount deposits must be rejected (`amount > 0` guard).
    let result = ctx
        .execute_instruction(deposit_ix(vault, authority.pubkey(), 0), &[&authority])
        .unwrap();
    result.assert_failure();

    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.balance, 0);
}

#[test]
fn withdraw_zero_amount_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &authority);

    ctx.execute_instruction(deposit_ix(vault, authority.pubkey(), 1_000), &[&authority])
        .unwrap()
        .assert_success();

    // Zero-amount withdrawals must be rejected.
    let result = ctx
        .execute_instruction(withdraw_ix(vault, authority.pubkey(), 0), &[&authority])
        .unwrap();
    result.assert_failure();

    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.balance, 1_000);
}

#[test]
fn pay_zero_amount_reverts() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let recipient = ctx.create_funded_account(0).unwrap();

    let result = ctx
        .execute_instruction(pay_ix(authority.pubkey(), recipient.pubkey(), 0), &[&authority])
        .unwrap();
    result.assert_failure();
}

#[test]
fn withdraw_exact_balance_succeeds() {
    let mut ctx = setup();
    let authority = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &authority);

    ctx.execute_instruction(deposit_ix(vault, authority.pubkey(), 1_000), &[&authority])
        .unwrap()
        .assert_success();

    // Withdrawing exactly the recorded balance must succeed (`<=` boundary).
    ctx.execute_instruction(withdraw_ix(vault, authority.pubkey(), 1_000), &[&authority])
        .unwrap()
        .assert_success();

    let v: demo_vault::Vault = ctx.get_account(&vault).unwrap();
    assert_eq!(v.balance, 0);
}

#[test]
fn close_succeeds_for_owner_and_transfers_lamports() {
    let mut ctx = setup();
    let owner = ctx.create_funded_account(10_000_000_000).unwrap();
    let vault = ensure_vault(&mut ctx, &owner);

    ctx.execute_instruction(deposit_ix(vault, owner.pubkey(), 2_000), &[&owner])
        .unwrap()
        .assert_success();

    let vault_lamports_before = ctx.svm.get_account(&vault).unwrap().lamports;
    let owner_lamports_before = ctx.svm.get_account(&owner.pubkey()).unwrap().lamports;

    ctx.execute_instruction(close_ix(vault, owner.pubkey()), &[&owner])
        .unwrap()
        .assert_success();

    // The vault is closed: its lamports went to the owner and the account
    // data was wiped (LiteSVM keeps a zeroed entry; the close semantic is
    // lamports == 0, not entry absence).
    let closed = ctx.svm.get_account(&vault).unwrap();
    assert_eq!(closed.lamports, 0);
    let owner_lamports_after = ctx.svm.get_account(&owner.pubkey()).unwrap().lamports;
    // The owner receives the vault's lamports minus the deterministic 5000
    // LiteSVM base fee (the owner signs the close transaction).
    assert_eq!(
        owner_lamports_after - owner_lamports_before,
        vault_lamports_before - 5_000
    );
}
