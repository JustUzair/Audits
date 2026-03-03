use crate::contexts::UpdateMarketParams;
use anchor_lang::prelude::*;

/// Update market parameters
pub fn update_market_params(
    ctx: Context<UpdateMarketParams>,
    new_collateral_factor: u64,
    new_liquidation_threshold: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;

    //@audit-note missing check if the authority is the market admin

    // Update market parameters
    market.collateral_factor = new_collateral_factor;
    market.liquidation_threshold = new_liquidation_threshold;

    msg!("Market parameters updated");
    Ok(())
}
