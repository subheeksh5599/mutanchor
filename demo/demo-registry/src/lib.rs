//! demo_registry — a second in-repo Anchor program used as an independent
//! mutation-testing target for Mutanchor.
//!
//! Different bug surface from `demo/demo-vault`:
//!   - name claim + PDA registration
//!   - authority-only transfer to a new owner
//!   - authority-only reclaim (close + rent refund)
//!   - authority-only pointer setter with checked arithmetic (upper bound)
//!
//! Every critical check lives on its own single line, so a mutation is one
//! clean, reported fault the operator suite can attribute to a specific
//! handler.

use anchor_lang::prelude::*;

declare_id!("DR2dYswVtGwYbZV8Jba83216igfDYzFqxkY1KZzLpQRm");

pub const MAX_POINTER: u64 = 1_000_000;

#[program]
pub mod demo_registry {
    use super::*;

    /// Register a `name` for the caller. Creates a PDA record with the
    /// caller as the initial owner and pointer=0.
    pub fn claim(ctx: Context<Claim>, name: String) -> Result<()> {
        require!(!name.is_empty(), DemoRegistryError::EmptyName);
        require!(name.len() <= 32, DemoRegistryError::NameTooLong);
        let record = &mut ctx.accounts.record;
        record.owner = ctx.accounts.authority.key();
        record.name = name;
        record.pointer = 0;
        Ok(())
    }

    /// Transfer ownership of `name` from the current authority to a
    /// `new_owner`. Only the current owner may call this.
    pub fn transfer(ctx: Context<Transfer>, _name: String, new_owner: Pubkey) -> Result<()> {
        require_keys_eq!(ctx.accounts.record.owner, ctx.accounts.authority.key(), DemoRegistryError::Unauthorized);
        ctx.accounts.record.owner = new_owner;
        Ok(())
    }

    /// Set the `pointer` field, bounded by MAX_POINTER. Uses `checked_add`
    /// so mutanchor's `unchecked_math` operator has a real target.
    pub fn set_pointer(ctx: Context<SetPointer>, _name: String, delta: u64) -> Result<()> {
        require_keys_eq!(ctx.accounts.record.owner, ctx.accounts.authority.key(), DemoRegistryError::Unauthorized);
        let next = ctx.accounts.record.pointer.checked_add(delta).ok_or(DemoRegistryError::Overflow)?;
        require!(next <= MAX_POINTER, DemoRegistryError::PointerOutOfRange);
        ctx.accounts.record.pointer = next;
        Ok(())
    }

    /// Close the record, returning rent to the current owner. Only the
    /// current owner may reclaim. (Takes a dummy `tag` so the generated
    /// instruction args struct is a plain, non-empty type.)
    pub fn reclaim(ctx: Context<Reclaim>, _name: String, _tag: u8) -> Result<()> {
        require_keys_eq!(ctx.accounts.record.owner, ctx.accounts.authority.key(), DemoRegistryError::Unauthorized);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Claim<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + NameRecord::INIT_SPACE,
        seeds = [b"record", name.as_bytes()],
        bump
    )]
    pub record: Account<'info, NameRecord>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Transfer<'info> {
    #[account(
        mut,
        seeds = [b"record", name.as_bytes()],
        bump
    )]
    pub record: Account<'info, NameRecord>,
    /// SIGNER: only the current owner may transfer.
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct SetPointer<'info> {
    #[account(
        mut,
        seeds = [b"record", name.as_bytes()],
        bump
    )]
    pub record: Account<'info, NameRecord>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Reclaim<'info> {
    #[account(
        mut,
        seeds = [b"record", name.as_bytes()],
        bump,
        close = authority
    )]
    pub record: Account<'info, NameRecord>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct NameRecord {
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub pointer: u64,
}

#[error_code]
pub enum DemoRegistryError {
    #[msg("name must not be empty")]
    EmptyName,
    #[msg("name must be at most 32 bytes")]
    NameTooLong,
    #[msg("unauthorized")]
    Unauthorized,
    #[msg("arithmetic overflow")]
    Overflow,
    #[msg("pointer out of range")]
    PointerOutOfRange,
}
