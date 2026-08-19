//! demo_vault — a deliberately small Anchor vault program used to exercise the
//! Mutanchor mutation-testing engine.
//!
//! Every handler is written with the audit-relevant bug-class patterns the
//! engine's operators target, and each critical check lives on its own single
//! line so a mutation is one clean, reported fault:
//!   - signer validation      (`Signer<'info>`, `#[account(signer)]`)
//!   - authority/owner check  (`require_keys_eq!` on one line)
//!   - PDA seeds + bump       (`seeds = [b"vault", authority.key().as_ref()]`)
//!   - boundary comparison    (`amount > 0`, `amount <= balance`)
//!   - close constraint       (`close = authority`)
//!   - CPI                    (`system_program::cpi::transfer`)

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("6xebaTYCQxo6Hsrs9mLh2Z3b2YH3bySw971U4ZDTEFp7");

#[program]
pub mod demo_vault {
    use super::*;

    /// Create (zero-balance) the caller's PDA vault, recording its owner.
    pub fn create(ctx: Context<Create>) -> Result<()> {
        ctx.accounts.vault.owner = ctx.accounts.authority.key();
        Ok(())
    }

    /// Deposit lamports into the caller's PDA vault.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, DemoVaultError::ZeroAmount);
        require_keys_eq!(ctx.accounts.vault.owner, ctx.accounts.authority.key(), DemoVaultError::Unauthorized);
        let vault: &mut Account<'_, Vault> = &mut ctx.accounts.vault;
        vault.balance = vault.balance.checked_add(amount).ok_or(DemoVaultError::Overflow)?;
        Ok(())
    }

    /// Withdraw, but never more than the recorded balance. Lamport movement
    /// is abstracted as a bookkeeping `balance` field; the real transfer is
    /// demonstrated by the separate `pay` instruction below (a system-program
    /// CPI cannot pull from an account that carries data).
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount <= ctx.accounts.vault.balance, DemoVaultError::Insufficient);
        require!(amount > 0, DemoVaultError::ZeroAmount);
        require_keys_eq!(ctx.accounts.vault.owner, ctx.accounts.authority.key(), DemoVaultError::Unauthorized);
        let vault: &mut Account<'_, Vault> = &mut ctx.accounts.vault;
        vault.balance = vault.balance.checked_sub(amount).ok_or(DemoVaultError::Overflow)?;
        Ok(())
    }

    /// Pay `amount` lamports from the authority to a recipient via a
    /// system-program CPI. Demonstrates the cpi-target pattern the engine's
    /// `cpi_target_swap` operator models.
    pub fn pay(ctx: Context<Pay>, amount: u64) -> Result<()> {
        require!(amount > 0, DemoVaultError::ZeroAmount);
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                },
            ),
            amount,
        )?;
        Ok(())
    }

    /// Close the vault, sending the remaining lamports to the authority.
    /// (Takes a dummy `tag` so the generated instruction args struct is a
    /// plain, non-empty type that the test suite can build directly.)
    pub fn close(ctx: Context<CloseAccounts>, _tag: u8) -> Result<()> {
        require_keys_eq!(ctx.accounts.vault.owner, ctx.accounts.authority.key(), DemoVaultError::Unauthorized);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Vault::INIT_SPACE,
        seeds = [b"vault", authority.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    /// SIGNER: only the vault authority may deposit.
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    /// SIGNER: only the vault authority may withdraw.
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Pay<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// SIGNER: recipient does not need to sign for a transfer to it.
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseAccounts<'info> {
    #[account(
        mut,
        seeds = [b"vault", authority.key().as_ref()],
        bump,
        close = authority
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub owner: Pubkey,
    pub balance: u64,
}

#[error_code]
pub enum DemoVaultError {
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("insufficient balance")]
    Insufficient,
    #[msg("overflow")]
    Overflow,
    #[msg("unauthorized")]
    Unauthorized,
}
