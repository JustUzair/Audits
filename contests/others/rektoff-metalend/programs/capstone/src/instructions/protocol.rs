use crate::contexts::InitializeProtocol;
use anchor_lang::prelude::*;

/// Initialize the global lending protocol state
pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;
    protocol_state.admin = ctx.accounts.admin.key();
    protocol_state.total_markets = 0;
    protocol_state.is_paused = false;
    protocol_state.bump = ctx.bumps.protocol_state;

    msg!(
        "MetaLend protocol initialized by admin: {}",
        ctx.accounts.admin.key()
    );
    Ok(())
}
