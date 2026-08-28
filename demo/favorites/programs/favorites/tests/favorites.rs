//! Rust LiteSVM test suite for the third-party favorites Anchor program.
//!
//! This is the Mutanchor kill-path harness for an unmodified third-party
//! program (the canonical `favorites` example from
//! `solana-developers/program-examples/basics/favorites/anchor`).
//!
//! The program .so is sourced from MUTANCHOR_PROGRAM_SO (set by the parent
//! Mutanchor runner) with a fallback to the crate-relative build output.

use anchor_lang::InstructionData;
use anchor_litesvm::AnchorLiteSVM;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;

use favorites::ID as PROGRAM_ID;

const SYSTEM_PROGRAM_ID: Pubkey = solana_program::pubkey!("11111111111111111111111111111111");

fn program_elf() -> Vec<u8> {
    let path = std::env::var("MUTANCHOR_PROGRAM_SO").unwrap_or_else(|_| {
        // Fallback: crate-local build.
        format!("{}/../../target/deploy/favorites.so", env!("CARGO_MANIFEST_DIR"))
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

fn favorites_pda(user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"favorites", user.as_ref()], &PROGRAM_ID).0
}

fn set_favorites_ix(
    favorites: Pubkey,
    user: Pubkey,
    number: u64,
    color: String,
    hobbies: Vec<String>,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(favorites, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: favorites::instruction::SetFavorites {
            number,
            color,
            hobbies,
        }
        .data(),
    }
}

#[test]
fn set_favorites_persists_state() {
    let mut ctx = setup();
    let user = ctx.create_funded_account(10_000_000_000).unwrap();
    let fav = favorites_pda(&user.pubkey());

    ctx.execute_instruction(
        set_favorites_ix(
            fav,
            user.pubkey(),
            42,
            "amber".to_string(),
            vec!["rust".to_string(), "solana".to_string()],
        ),
        &[&user],
    )
    .unwrap()
    .assert_success();

    let r: favorites::Favorites = ctx.get_account(&fav).unwrap();
    assert_eq!(r.number, 42);
    assert_eq!(r.color, "amber");
    assert_eq!(r.hobbies, vec!["rust".to_string(), "solana".to_string()]);
}

#[test]
fn set_favorites_updates_existing_record() {
    let mut ctx = setup();
    let user = ctx.create_funded_account(10_000_000_000).unwrap();
    let fav = favorites_pda(&user.pubkey());

    // Initial claim.
    ctx.execute_instruction(
        set_favorites_ix(fav, user.pubkey(), 1, "red".to_string(), vec![]),
        &[&user],
    )
    .unwrap()
    .assert_success();

    // Second call (`init_if_needed` allows updates).
    ctx.execute_instruction(
        set_favorites_ix(fav, user.pubkey(), 99, "blue".to_string(), vec!["chess".to_string()]),
        &[&user],
    )
    .unwrap()
    .assert_success();

    let r: favorites::Favorites = ctx.get_account(&fav).unwrap();
    assert_eq!(r.number, 99);
    assert_eq!(r.color, "blue");
    assert_eq!(r.hobbies, vec!["chess".to_string()]);
}

#[test]
fn different_users_get_distinct_pdas() {
    let mut ctx = setup();
    let alice = ctx.create_funded_account(10_000_000_000).unwrap();
    let bob = ctx.create_funded_account(10_000_000_000).unwrap();

    let alice_fav = favorites_pda(&alice.pubkey());
    let bob_fav = favorites_pda(&bob.pubkey());
    assert_ne!(alice_fav, bob_fav);

    ctx.execute_instruction(
        set_favorites_ix(alice_fav, alice.pubkey(), 1, "red".to_string(), vec![]),
        &[&alice],
    )
    .unwrap()
    .assert_success();
    ctx.execute_instruction(
        set_favorites_ix(bob_fav, bob.pubkey(), 2, "green".to_string(), vec![]),
        &[&bob],
    )
    .unwrap()
    .assert_success();

    let a: favorites::Favorites = ctx.get_account(&alice_fav).unwrap();
    let b: favorites::Favorites = ctx.get_account(&bob_fav).unwrap();
    assert_eq!(a.number, 1);
    assert_eq!(b.number, 2);
}
