use crate::{
    contexts::Withdraw,
    utils::{
        calculate_exchange_rate, calculate_underlying_from_ctokens, get_asset_price,
        update_market_interest,
    },
    LendingError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Transfer};

/// Withdraw supplied tokens (burn cTokens)
pub fn withdraw(ctx: Context<Withdraw>, market_id: u64, ctoken_amount: u64) -> Result<()> {
    let market_account_info = ctx.accounts.market.to_account_info();
    let market = &mut ctx.accounts.market;
    let user_deposit = &mut ctx.accounts.user_deposit;

    update_market_interest(market)?;

    // Calculate proper exchange rate based on accumulated interest
    let exchange_rate = calculate_exchange_rate(market)?;

    // Calculate how many underlying tokens to return
    // @note amount * rate / factor
    let tokens_to_withdraw =
        calculate_underlying_from_ctokens(ctoken_amount as u128, exchange_rate)?;

    require!(
        user_deposit.ctoken_balance >= (ctoken_amount as u128),
        LendingError::InsufficientBalance
    );

    // Check if account remains properly collateralized after withdrawal
    // Get prices from separate oracles
    // @audit-note oracle comes in from user's withdraw "account" instruction, can the oracle be arbitrarily passed
    let supply_price = get_asset_price(&ctx.accounts.supply_oracle)?;
    let collateral_price = get_asset_price(&ctx.accounts.collateral_oracle)?;

    // Calculate collateral value
    let collateral_value = user_deposit
        .collateral_deposited
        .checked_mul(collateral_price)
        .ok_or(LendingError::MathOverflow)?;

    // Calculate borrow value in supply asset terms
    let borrow_value = user_deposit
        .borrowed_amount
        .checked_mul(supply_price)
        .ok_or(LendingError::MathOverflow)?;

    // Calculate maximum allowed borrow based on collateral
    let max_borrow_value = collateral_value
        .checked_mul(market.collateral_factor as u128)
        .and_then(|v| v.checked_div(10000))
        .ok_or(LendingError::MathOverflow)?;

    require!(
        borrow_value <= max_borrow_value,
        LendingError::InsufficientCollateral
    );

    let market_bump = market.bump;
    let supply_mint = ctx.accounts.supply_mint.key();
    let collateral_mint = ctx.accounts.collateral_mint.key();

    // Transfer supply tokens back to user from supply vault
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

    require!(
        tokens_to_withdraw <= u64::MAX as u128,
        LendingError::MathOverflow
    );

    token_interface::transfer(cpi_ctx, tokens_to_withdraw as u64)?;

    // Update balances
    // @audit-note  ctoken after accruing yeild (amount * rate / factor) is more valuable
    // @audit-note initially, 1000 USDC, 1000 cTokens -> 1 cToken = 1 USDC
    // @audit-note 50 USDC interest paid into the pool
    // @audit-note now 1050 USDC pool, 1000 cTokens ->  1 cToken = 1.05 USDC
    // @audit-note more value is subtracted from user's supply deposited, probably leading to underflow

    user_deposit.supply_deposited = user_deposit
        .supply_deposited
        .checked_sub(tokens_to_withdraw)
        .ok_or(LendingError::MathOverflow)?;
    user_deposit.ctoken_balance = user_deposit
        .ctoken_balance
        .checked_sub(ctoken_amount as u128)
        .ok_or(LendingError::MathOverflow)?;
    market.total_supply_deposits = market
        .total_supply_deposits
        .checked_sub(tokens_to_withdraw)
        .ok_or(LendingError::MathOverflow)?;

    // Update total cToken supply
    market.total_ctoken_supply = market
        .total_ctoken_supply
        .checked_sub(ctoken_amount as u128)
        .ok_or(LendingError::MathOverflow)?;

    msg!(
        "Withdraw successful: {} cTokens → {} tokens",
        ctoken_amount,
        tokens_to_withdraw
    );
    Ok(())
}
