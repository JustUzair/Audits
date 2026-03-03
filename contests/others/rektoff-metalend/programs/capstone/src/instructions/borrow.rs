use crate::{
    contexts::{Borrow, WithdrawCollateral},
    utils::{get_asset_price, update_market_interest, SCALING_FACTOR},
    LendingError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Transfer};

/// Borrow supply tokens by depositing collateral tokens
pub fn borrow(
    ctx: Context<Borrow>,
    market_id: u64,
    collateral_amount: u64,
    borrow_amount: u64,
) -> Result<()> {
    let market_account_info = ctx.accounts.market.to_account_info();
    let market = &mut ctx.accounts.market;
    let user_deposit = &mut ctx.accounts.user_deposit;

    update_market_interest(market)?;

    //@audit-note no check if the oracle supplied in Borrow account are the oracles used to create the market

    // Get asset prices from oracles, we use specific oracles for each asset to get the correct price
    let collateral_price = get_asset_price(&ctx.accounts.collateral_oracle)?;
    let borrow_price = get_asset_price(&ctx.accounts.borrow_oracle)?;

    // First, deposit the collateral tokens to collateral vault
    if collateral_amount > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_collateral_account.to_account_info(),
            to: ctx.accounts.collateral_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token_interface::transfer(cpi_ctx, collateral_amount)?;

        // Update collateral balance
        let collateral_amount_u128 = collateral_amount as u128;
        user_deposit.collateral_deposited = user_deposit
            .collateral_deposited
            .checked_add(collateral_amount_u128)
            .ok_or(LendingError::MathOverflow)?;

        market.total_collateral_deposits = market
            .total_collateral_deposits
            .checked_add(collateral_amount_u128)
            .ok_or(LendingError::MathOverflow)?;
    }

    // Calculate collateral value using the oracle price
    // u128 calculations prevent overflow issues
    let total_collateral_value = user_deposit
        .collateral_deposited
        .checked_mul(collateral_price)
        .ok_or_else(|| LendingError::MathOverflow)?;

    let collateral_factor_u128 = market.collateral_factor as u128;
    let max_borrow_value = total_collateral_value
        .checked_mul(collateral_factor_u128)
        .and_then(|v| v.checked_div(10000))
        .ok_or_else(|| LendingError::MathOverflow)?;

    let borrow_amount_u128 = borrow_amount as u128;
    let new_total_borrowed = user_deposit
        .borrowed_amount
        .checked_add(borrow_amount_u128)
        .ok_or_else(|| LendingError::MathOverflow)?;
    let new_borrow_value = new_total_borrowed
        .checked_mul(borrow_price)
        .ok_or_else(|| LendingError::MathOverflow)?;

    require!(
        new_borrow_value <= max_borrow_value,
        LendingError::InsufficientCollateral
    );

    let available_liquidity = market
        .total_supply_deposits
        .checked_sub(market.total_borrows)
        .unwrap_or(0);
    let borrow_amount_u128 = borrow_amount as u128;
    require!(
        borrow_amount_u128 <= available_liquidity,
        LendingError::InsufficientLiquidity
    );

    // Apply simple interest to existing borrows (2% annual rate)
    // [CAPSTONE_SAFE: Simplified interest accrual is intentional for educational purposes]
    if user_deposit.borrowed_amount > 0 {
        let current_slot = Clock::get()?.slot;
        /* ex:
        @audit-note
        user_deposit.rs >>>> users account is initialized at slot 1000 (last_update_slot)
        slot now = 10,000
        slots_elapsed = 10,000 - 1000 = 9,000

        interest increment = borrowed amount * interest rate per slot * (9,000)
        slots, but user didn't take out loan in slot 1000 only the account was initialized

         */
        let slots_elapsed = current_slot.saturating_sub(user_deposit.last_update_slot);
        let interest_rate_per_slot = 25u128; // ~2% annual / 800,000 slots
        let slots_elapsed_u128 = slots_elapsed as u128;
        let interest_increment = user_deposit
            .borrowed_amount
            .saturating_mul(interest_rate_per_slot)
            .saturating_mul(slots_elapsed_u128)
            / SCALING_FACTOR; // Scale down

        user_deposit.borrowed_amount = user_deposit
            .borrowed_amount
            .saturating_add(interest_increment);
        // @audit-note missing interest increment addition to the market.total_borrows
        user_deposit.last_update_slot = current_slot;
    }

    let market_bump = market.bump;
    let supply_mint = ctx.accounts.supply_mint.key();
    let collateral_mint = ctx.accounts.collateral_mint.key();

    // Transfer supply tokens to borrower from supply vault
    let market_id_bytes = market_id.to_le_bytes();
    let market_seeds = &[
        b"market",
        market_id_bytes.as_ref(),
        supply_mint.as_ref(),
        collateral_mint.as_ref(),
        &[market_bump],
    ];
    let signer_seeds = &[market_seeds.as_slice()];

    let cpi_accounts = Transfer {
        from: ctx.accounts.supply_vault.to_account_info(),
        to: ctx.accounts.user_supply_account.to_account_info(),
        authority: market_account_info,
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_interface::transfer(cpi_ctx, borrow_amount)?;

    // Update borrow balances
    let borrow_amount_u128 = borrow_amount as u128;
    user_deposit.borrowed_amount = user_deposit
        .borrowed_amount
        .checked_add(borrow_amount_u128)
        .ok_or(LendingError::MathOverflow)?;
    market.total_borrows = market
        .total_borrows
        .checked_add(borrow_amount_u128)
        .ok_or(LendingError::MathOverflow)?;

    msg!(
        "Borrow successful: {} collateral → {} supply tokens",
        collateral_amount,
        borrow_amount
    );
    Ok(())
}

/// Withdraw collateral tokens (only allowed when no outstanding borrows)
pub fn withdraw_collateral(
    ctx: Context<WithdrawCollateral>,
    market_id: u64,
    collateral_amount: u64,
) -> Result<()> {
    let user_deposit = &mut ctx.accounts.user_deposit;

    // Ensure user has no outstanding borrows
    require!(user_deposit.borrowed_amount == 0, LendingError::HasBorrows);

    // Ensure user has enough collateral
    require!(
        user_deposit.collateral_deposited >= collateral_amount as u128,
        LendingError::InsufficientBalance
    );

    // Get values needed for signer seeds before borrowing market
    let market_bump = ctx.accounts.market.bump;
    let supply_mint = ctx.accounts.supply_mint.key();
    let collateral_mint = ctx.accounts.collateral_mint.key();
    let market_id_bytes = market_id.to_le_bytes();
    let market_seeds = &[
        b"market".as_ref(),
        market_id_bytes.as_ref(),
        supply_mint.as_ref(),
        collateral_mint.as_ref(),
        &[market_bump],
    ];
    let signer_seeds = &[market_seeds.as_slice()];

    // Transfer collateral from vault back to user
    let cpi_accounts = Transfer {
        from: ctx.accounts.collateral_vault.to_account_info(),
        to: ctx.accounts.user_collateral_account.to_account_info(),
        authority: ctx.accounts.market.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_interface::transfer(cpi_ctx, collateral_amount)?;

    // Update user deposit balances
    user_deposit.collateral_deposited = user_deposit
        .collateral_deposited
        .checked_sub(collateral_amount as u128)
        .ok_or(LendingError::MathOverflow)?;

    // Update market totals
    let market = &mut ctx.accounts.market;
    market.total_collateral_deposits = market
        .total_collateral_deposits
        .checked_sub(collateral_amount as u128)
        .ok_or(LendingError::MathOverflow)?;

    msg!(
        "Collateral withdrawal successful: {} tokens",
        collateral_amount
    );
    Ok(())
}
