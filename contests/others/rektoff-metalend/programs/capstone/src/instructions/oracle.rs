use crate::contexts::{CreateOracle, UpdateOraclePrice};
use anchor_lang::prelude::*;

/// Initialize a new Oracle account
pub fn create_oracle(
    ctx: Context<CreateOracle>,
    source: Vec<u8>,
    initial_price: u64,
    decimals: u8,
) -> Result<()> {
    let oracle = &mut ctx.accounts.oracle;

    oracle.mint = ctx.accounts.mint.key();
    oracle.source = source;
    oracle.price = initial_price as u128;
    oracle.decimals = decimals;
    oracle.valid_slot = Clock::get()?.slot;
    oracle.confidence = 5; // Initial confidence set to 5%
    oracle.authority = ctx.accounts.authority.key();
    oracle.bump = ctx.bumps.oracle;

    Ok(())
}

/// Update oracle price - FOR TESTING/CAPSTONE PURPOSES ONLY [CAPSTONE_SAFE]
/// In production, this would be done by authorized price feeds like Pyth/Switchboard
pub fn update_oracle_price(ctx: Context<UpdateOraclePrice>, new_price: u64) -> Result<()> {
    let oracle = &mut ctx.accounts.oracle;
    let current_slot = Clock::get()?.slot;

    // Update price and timestamp
    oracle.price = new_price as u128;
    oracle.valid_slot = current_slot;
    oracle.confidence = (new_price / 100) as u128; // 1% confidence interval

    msg!(
        "Oracle price updated to: {} at slot: {}",
        new_price,
        current_slot
    );
    msg!("NOTE: This function is for testing/capstone purposes only!");
    msg!("In production, oracle updates would come from authorized price feeds!");
    Ok(())
}
