//! A minimal Anchor program used to exercise and unit-test the Mutanchor
//! engine. Deliberately small and full of the audit-relevant patterns the
//! operators target: signer checks, authority checks, PDA seeds + bump, and a
//! boundary comparison. This file is a fixture — the real demo program is in
//! `demo/demo-vault`.

use anchor_lang::prelude::*;

declare_id!("FIXTUR11111111111111111111111111111111111111");

#[program]
pub mod fixture_vault {
    use super::*;

    /// Deposit lamports into a PDA vault owned by the caller's authority.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, FixtureError::ZeroAmount);
        // AUTHORITY CHECK: only the vault owner may deposit.
        require!(
            ctx.accounts.authority.key() == ctx.accounts.vault.owner,
            FixtureError::Unauthorized
        );
        // SIGNER CHECK happens via the Signer<'_> account type.
        let vault: &mut Account<'_, Vault> = &mut ctx.accounts.vault;
        vault.balance += amount;
        Ok(())
    }

    /// Withdraw, but only up to the recorded balance (boundary check).
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount <= ctx.accounts.vault.balance, FixtureError::Insufficient);
        let vault: &mut Account<'_, Vault> = &mut ctx.accounts.vault;
        vault.balance -= amount;
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
    /// SIGNER CHECK: this field requires the caller to be a signer.
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump,
        close = authority
    )]
    pub vault: Account<'info, Vault>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub balance: u64,
}

#[error_code]
pub enum FixtureError {
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("unauthorized")]
    Unauthorized,
    #[msg("insufficient balance")]
    Insufficient,
}
