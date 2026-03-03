use crate::{
    contexts::Supply,
    utils::{calculate_ctokens_to_mint, calculate_exchange_rate, update_market_interest},
    LendingError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Transfer};

/// Supply tokens to earn interest (mint cTokens)
pub fn supply(ctx: Context<Supply>, _market_id: u64, amount: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let user_deposit = &mut ctx.accounts.user_deposit;

    // Update interest first
    update_market_interest(market)?;

    // Calculate proper exchange rate based on accumulated interest
    let exchange_rate = calculate_exchange_rate(market)?;
    let ctokens_to_mint = calculate_ctokens_to_mint(amount, exchange_rate)?;

    // Transfer supply tokens from user to supply vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_supply_account.to_account_info(),
        to: ctx.accounts.supply_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_interface::transfer(cpi_ctx, amount)?;

    // Update balances
    // @audit-note only updated here
    // @audit-note user deposits their supply tokens and get ctoken, mostly 1:1 ratio
    // @audit-note supply deposited is only set here
    user_deposit.supply_deposited = user_deposit
        .supply_deposited
        .checked_add(amount as u128)
        .ok_or(LendingError::MathOverflow)?;

    user_deposit.ctoken_balance = user_deposit
        .ctoken_balance
        .checked_add(ctokens_to_mint)
        .ok_or(LendingError::MathOverflow)?;

    market.total_supply_deposits = market
        .total_supply_deposits
        .checked_add(amount as u128)
        .ok_or(LendingError::MathOverflow)?;

    // Track total cToken supply
    market.total_ctoken_supply = market
        .total_ctoken_supply
        .checked_add(ctokens_to_mint)
        .ok_or(LendingError::MathOverflow)?;

    msg!(
        "Supply successful: {} tokens → {} cTokens",
        amount,
        ctokens_to_mint
    );
    Ok(())
}
