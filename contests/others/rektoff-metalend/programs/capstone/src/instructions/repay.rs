use crate::{
    contexts::Repay,
    utils::{update_market_interest, SCALING_FACTOR},
    LendingError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Transfer};
use std::cmp;

/// Repay borrowed tokens
pub fn repay(ctx: Context<Repay>, _market_id: u64, amount: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let user_deposit = &mut ctx.accounts.user_deposit;

    update_market_interest(market)?;

    // Apply simple interest to existing debt (2% annual rate)
    // [CAPSTONE_SAFE: Simplified interest accrual is intentional for educational purposes]
    if user_deposit.borrowed_amount > 0 {
        let current_slot = Clock::get()?.slot;
        let slots_elapsed = current_slot.saturating_sub(user_deposit.last_update_slot);
        // @audit-note incorrect rate per slot
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

        // @audit-note missing intereset increment addition to the market.total_borrows
        user_deposit.last_update_slot = current_slot;
    }

    // @audit-q why is the user not allowed to pay the borrowed amount in full?
    let repay_amount_u128 = cmp::min(amount as u128, user_deposit.borrowed_amount);
    require!(
        repay_amount_u128 <= u64::MAX as u128,
        LendingError::MathOverflow
    );
    let repay_amount = repay_amount_u128 as u64;

    // Transfer supply tokens from user to supply vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_supply_account.to_account_info(),
        to: ctx.accounts.supply_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_interface::transfer(cpi_ctx, repay_amount)?;

    // Update balances
    user_deposit.borrowed_amount = user_deposit
        .borrowed_amount
        .checked_sub(repay_amount as u128)
        .ok_or(LendingError::MathOverflow)?;
    market.total_borrows = market
        .total_borrows
        .checked_sub(repay_amount as u128)
        .ok_or(LendingError::MathOverflow)?;

    msg!("Repay successful: {} tokens", repay_amount);
    Ok(())
}
