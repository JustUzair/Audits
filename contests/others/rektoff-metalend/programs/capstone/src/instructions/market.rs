use crate::contexts::CreateMarket;
use crate::utils::SCALING_FACTOR;
use anchor_lang::prelude::*;

/// Create a new lending market with separate supply and collateral assets
pub fn create_market(
    ctx: Context<CreateMarket>,
    market_id: u64,
    collateral_factor: u64, // Basis points (e.g., 8000 = 80%)
    liquidation_threshold: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let protocol_state = &mut ctx.accounts.protocol_state;

    market.market_id = market_id;
    market.supply_mint = ctx.accounts.supply_mint.key();
    market.collateral_mint = ctx.accounts.collateral_mint.key();
    market.market_admin = ctx.accounts.creator.key();
    market.total_supply_deposits = 0;
    market.total_borrows = 0;
    market.total_collateral_deposits = 0;
    market.total_ctoken_supply = 0;
    market.collateral_factor = collateral_factor;
    market.liquidation_threshold = liquidation_threshold;
    market.last_update_slot = Clock::get()?.slot;
    market.cumulative_borrow_rate = SCALING_FACTOR;
    market.cumulative_supply_rate = SCALING_FACTOR;
    market.supply_oracle = ctx.accounts.supply_oracle.key();
    market.collateral_oracle = ctx.accounts.collateral_oracle.key();
    market.bump = ctx.bumps.market;
    market.is_active = true;

    protocol_state.total_markets += 1;

    msg!(
        "Market created - Supply: {}, Collateral: {}, ID: {}",
        market.supply_mint,
        market.collateral_mint,
        market_id
    );
    Ok(())
}
