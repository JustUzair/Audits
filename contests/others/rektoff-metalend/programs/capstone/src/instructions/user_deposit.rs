use crate::{
    contexts::{CloseUserDeposit, InitializeUserDeposit},
    LendingError, UserDeposit,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::rent::Rent;
use anchor_lang::solana_program::sysvar::Sysvar;
use anchor_lang::system_program;

/// Initialize user deposit account using raw Solana account creation
pub fn initialize_user_deposit(ctx: Context<InitializeUserDeposit>, market_id: u64) -> Result<()> {
    let user_deposit_info = &ctx.accounts.user_deposit;

    // Check if account already exists
    if user_deposit_info.lamports() > 0 {
        return Err(LendingError::UserDepositAlreadyExists.into());
    }

    // Generate PDA and verify
    let (expected_pda, bump) = Pubkey::find_program_address(
        &[
            b"user_deposit",
            ctx.accounts.user.key().as_ref(),
            market_id.to_le_bytes().as_ref(),
            ctx.accounts.supply_mint.key().as_ref(),
            ctx.accounts.collateral_mint.key().as_ref(),
        ],
        ctx.program_id,
    );

    if expected_pda != user_deposit_info.key() {
        return Err(LendingError::InvalidPDA.into());
    }

    // Calculate space and rent
    let space = UserDeposit::SPACE;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);

    // Create account using CPI
    let cpi_accounts = system_program::CreateAccount {
        from: ctx.accounts.user.to_account_info(),
        to: user_deposit_info.to_account_info(),
    };

    let market_id_bytes = market_id.to_le_bytes();
    let bump_bytes = [bump];
    let user_key = ctx.accounts.user.key();
    let supply_mint_key = ctx.accounts.supply_mint.key();
    let collateral_mint_key = ctx.accounts.collateral_mint.key();
    let seeds: &[&[u8]] = &[
        b"user_deposit",
        user_key.as_ref(),
        market_id_bytes.as_ref(),
        supply_mint_key.as_ref(),
        collateral_mint_key.as_ref(),
        bump_bytes.as_ref(),
    ];
    let signer_seeds = [seeds];

    let cpi_program = ctx.accounts.system_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer_seeds);

    system_program::create_account(cpi_ctx, lamports, space as u64, ctx.program_id)?;

    // Initialize account data
    let mut account_data = user_deposit_info.try_borrow_mut_data()?;

    let discriminator = <UserDeposit as anchor_lang::Discriminator>::DISCRIMINATOR;
    account_data[0..8].copy_from_slice(&discriminator);

    let user_deposit_data = UserDeposit {
        user: ctx.accounts.user.key(),
        market: ctx.accounts.market.key(),
        supply_deposited: 0,
        collateral_deposited: 0,
        borrowed_amount: 0,
        ctoken_balance: 0,
        last_update_slot: Clock::get()?.slot,
        bump,
    };

    let serialized = user_deposit_data.try_to_vec()?;
    account_data[8..8 + serialized.len()].copy_from_slice(&serialized);

    msg!("User deposit account created for market: {}", market_id);
    Ok(())
}

/// Close user deposit account
pub fn close_user_deposit(ctx: Context<CloseUserDeposit>) -> Result<()> {
    let user_deposit = &ctx.accounts.user_deposit;

    // Check that account has no deposits or borrows
    require!(
        user_deposit.supply_deposited == 0 && user_deposit.collateral_deposited == 0,
        LendingError::HasDeposits
    );
    require!(user_deposit.borrowed_amount == 0, LendingError::HasBorrows);

    // Transfer lamports back to user
    let user_deposit_info = ctx.accounts.user_deposit.to_account_info();
    let user_info = ctx.accounts.user.to_account_info();

    let lamports = user_deposit_info.lamports();
    **user_deposit_info.try_borrow_mut_lamports()? = 0;
    **user_info.try_borrow_mut_lamports()? = user_info
        .lamports()
        .checked_add(lamports)
        .ok_or(LendingError::MathOverflow)?;

    msg!(
        "User deposit account closed, {} lamports returned",
        lamports
    );
    Ok(())
}
