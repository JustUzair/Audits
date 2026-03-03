use crate::{
    contexts::Liquidate,
    utils::{get_asset_price, update_market_interest},
    LendingError,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Transfer};

/// Liquidate undercollateralized positions
pub fn liquidate(ctx: Context<Liquidate>, market_id: u64, liquidation_amount: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let borrower_deposit = &mut ctx.accounts.borrower_deposit;

    require!(
        borrower_deposit.market == market.key(),
        LendingError::InvalidPDA
    );

    update_market_interest(market)?;

    // Check if position is liquidatable
    // @audit-note only one oracle to price both values
    let asset_price = get_asset_price(&ctx.accounts.oracle)?;
    let collateral_value = borrower_deposit
        .collateral_deposited
        .checked_mul(asset_price)
        .ok_or(LendingError::MathOverflow)?;
    let borrow_value = borrower_deposit
        .borrowed_amount
        .checked_mul(asset_price)
        .ok_or(LendingError::MathOverflow)?;
    let liquidation_threshold_value = collateral_value
        .checked_mul(market.liquidation_threshold as u128)
        .and_then(|v| v.checked_div(10000))
        .ok_or(LendingError::MathOverflow)?;

    require!(
        borrow_value > liquidation_threshold_value,
        LendingError::PositionHealthy
    );

    // Calculate liquidation bonus
    let liquidation_bonus = 1100; // 10% bonus
    let collateral_to_seize = liquidation_amount * liquidation_bonus / 1000;

    // Validate liquidation amount
    require!(
        (liquidation_amount as u128) <= borrower_deposit.borrowed_amount,
        LendingError::ExcessiveLiquidation
    );

    // Transfer repayment from liquidator to supply vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.liquidator_supply_account.to_account_info(),
        to: ctx.accounts.supply_vault.to_account_info(),
        authority: ctx.accounts.liquidator.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_interface::transfer(cpi_ctx, liquidation_amount)?;

    let supply_mint = ctx.accounts.supply_mint.key();
    let collateral_mint = ctx.accounts.collateral_mint.key();
    // Transfer collateral to liquidator
    let market_id_bytes = market_id.to_le_bytes();
    let market_seeds = &[
        b"market",
        market_id_bytes.as_ref(),
        supply_mint.as_ref(),
        collateral_mint.as_ref(),
        &[market.bump],
    ];
    let signer_seeds = &[market_seeds.as_slice()];

    let cpi_accounts = Transfer {
        from: ctx.accounts.collateral_vault.to_account_info(),
        to: ctx.accounts.liquidator_collateral_account.to_account_info(),
        authority: ctx.accounts.market.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_interface::transfer(cpi_ctx, collateral_to_seize)?;

    // @audit-note market total borrows and total collateral deposits are not updated.
    // Update borrower balances
    borrower_deposit.borrowed_amount -= liquidation_amount as u128;
    borrower_deposit.collateral_deposited -= collateral_to_seize as u128;

    msg!(
        "Liquidation successful: {} debt → {} collateral",
        liquidation_amount,
        collateral_to_seize
    );
    Ok(())
}
