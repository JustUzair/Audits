use crate::{LendingError, Market, Oracle};
use anchor_lang::prelude::*;

/// Scaling factor for exchange rate calculations (1e9)
/// This is used to scale the exchange rate to a whole number
pub const SCALING_FACTOR: u128 = 1_000_000_000;

/// Update market interest rates with simple flat rates: 1% supply, 2% borrow
/// [CAPSTONE_SAFE: Simplified flat-rate interest accrual is intentional for educational purposes]
pub fn update_market_interest(market: &mut Market) -> Result<()> {
    let current_slot = Clock::get()?.slot;
    let slots_elapsed = current_slot - market.last_update_slot;

    if slots_elapsed == 0 {
        return Ok(());
    }

    // @audit-note verify the slots and the comments and understand if its as intended
    // Simple flat rates: 2% borrow, 1% supply (annual rates)
    // Convert to per-slot rates (very small increments)
    // Assuming ~800,000 slots per year (400ms slots), rates per slot:
    let borrow_rate_per_slot = 25u128; // ~2% annual / 800,000 slots * 1e9 scale
    let supply_rate_per_slot = 12u128; // ~1% annual / 800,000 slots * 1e9 scale

    // Limit slots to prevent any overflow (max 1 day worth of slots)
    let slots_elapsed = slots_elapsed.min(216000); // ~1 day of slots
    let slots_elapsed_u128 = slots_elapsed as u128;

    // Update cumulative borrow rate
    let borrow_increment = borrow_rate_per_slot * slots_elapsed_u128;
    market.cumulative_borrow_rate = market
        .cumulative_borrow_rate
        .saturating_add(borrow_increment);

    // Update cumulative supply rate
    let supply_increment = supply_rate_per_slot * slots_elapsed_u128;
    market.cumulative_supply_rate = market
        .cumulative_supply_rate
        .saturating_add(supply_increment);

    // [CAPSTONE_SAFE: Mock interest — add a tiny amount to total_supply_deposits so cTokens appreciate]
    if market.total_supply_deposits > 0 {
        let interest_earned = market.total_supply_deposits / 100000; // 0.001% per interaction
        market.total_supply_deposits = market.total_supply_deposits.saturating_add(interest_earned);
    }

    market.last_update_slot = current_slot;
    Ok(())
}

pub fn update_market_interest_readonly(_market: &Market) -> Result<()> {
    // Dummy function for read-only operations
    Ok(())
}

/// Get asset price from oracle with proper validation
pub fn get_asset_price(oracle_account: &AccountInfo) -> Result<u128> {
    // Deserialize oracle account
    let oracle = Oracle::try_deserialize(&mut &oracle_account.data.borrow()[..])?;
    // Check if oracle data is still valid (within 100 slots)
    let current_slot = Clock::get()?.slot;
    if !oracle.is_valid(current_slot, 100) {
        msg!("Oracle data is stale");
        return Err(LendingError::InvalidOracleData.into());
    }

    // Additional validation: check confidence is within acceptable bounds
    // Reject price if confidence interval is too wide (>5% of price)
    if oracle.confidence > oracle.price / 20 {
        msg!(
            "Oracle confidence too wide: {} > {}",
            oracle.confidence,
            oracle.price / 20
        );
        return Err(LendingError::InvalidOracleData.into());
    }

    Ok(oracle.price)
}

/// Calculate exchange rate for cTokens - simplified version
pub fn calculate_exchange_rate(market: &Market) -> Result<u128> {
    if market.total_ctoken_supply == 0 || market.total_supply_deposits == 0 {
        return Ok(SCALING_FACTOR); // 1:1 initial rate
    }

    // Simple exchange rate: total_supply_deposits / total_ctoken_supply
    // This naturally appreciates as interest is added to total_supply_deposits
    let scaled_deposits = market.total_supply_deposits.checked_mul(SCALING_FACTOR);
    let exchange_rate = scaled_deposits
        .and_then(|v| v.checked_div(market.total_ctoken_supply))
        .unwrap_or(SCALING_FACTOR);

    // Ensure rate never goes below 1:1
    let final_rate = if exchange_rate > SCALING_FACTOR {
        exchange_rate
    } else {
        SCALING_FACTOR
    };
    Ok(final_rate)
}

/// Calculate how many cTokens to mint for a given supply amount
pub fn calculate_ctokens_to_mint(supply_amount: u64, exchange_rate: u128) -> Result<u128> {
    let supply_amount_u128 = supply_amount as u128;
    let numerator = supply_amount_u128
        .checked_mul(SCALING_FACTOR)
        .ok_or(LendingError::MathOverflow)?;

    let ctokens = numerator
        .checked_add(exchange_rate)
        .and_then(|v| v.checked_sub(1))
        .and_then(|v| v.checked_div(exchange_rate))
        .ok_or(LendingError::MathOverflow)?;

    Ok(ctokens)
}

/// Calculate how many underlying tokens to return for cToken redemption
pub fn calculate_underlying_from_ctokens(ctoken_amount: u128, exchange_rate: u128) -> Result<u128> {
    ctoken_amount
        .checked_mul(exchange_rate)
        .and_then(|v| v.checked_div(SCALING_FACTOR))
        .ok_or(LendingError::MathOverflow.into())
}

/// Calculate health factor for liquidation
pub fn calculate_health_factor(
    collateral_value: u128,
    borrow_value: u128,
    liquidation_threshold: u64,
) -> Result<u128> {
    if borrow_value == 0 {
        return Ok(u128::MAX); // Infinite health factor
    }

    let liquidation_threshold_u128 = liquidation_threshold as u128;
    let threshold_value = collateral_value * liquidation_threshold_u128 / 10000;

    // Return health factor scaled by 1e9
    Ok(threshold_value * SCALING_FACTOR / borrow_value)
}

/// Check if position is liquidatable
pub fn is_liquidatable(
    collateral_value: u128,
    borrow_value: u128,
    liquidation_threshold: u64,
) -> bool {
    let liquidation_threshold_u128 = liquidation_threshold as u128;
    let threshold_value = collateral_value * liquidation_threshold_u128 / 10000;
    borrow_value > threshold_value
}

/// Calculate maximum borrowable amount
pub fn calculate_max_borrow(
    collateral_value: u128,
    existing_borrow_value: u128,
    collateral_factor: u64,
) -> u128 {
    let collateral_factor_u128 = collateral_factor as u128;
    let max_borrow_value = collateral_value * collateral_factor_u128 / 10000;
    if max_borrow_value > existing_borrow_value {
        max_borrow_value - existing_borrow_value
    } else {
        0
    }
}
