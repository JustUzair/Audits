use crate::state::{Market, Oracle, ProtocolState, UserDeposit};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    // @note protocol state can only be initialized once
    #[account(
        init,
        payer = admin,
        space = ProtocolState::SPACE,
        seeds = [b"protocol"], // @audit-note simple seed, is this used anywhere that can cause problem?
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    // @note admin account is mutable
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
// @note market id is externally supplied
#[instruction(market_id: u64)]
pub struct CreateMarket<'info> {
    //@note market is init once
    #[account(
        init,
        payer = creator,
        space = Market::space(),
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    #[account(mut)]
    pub protocol_state: Account<'info, ProtocolState>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: Oracle account for supply asset pricing
    pub supply_oracle: AccountInfo<'info>,
    /// CHECK: Oracle account for collateral pricing
    pub collateral_oracle: AccountInfo<'info>,
    #[account(
        init,
        payer = creator,
        token::mint = supply_mint,
        token::authority = market,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = creator,
        token::mint = collateral_mint,
        token::authority = market,
        seeds = [b"collateral_vault", market_id.to_le_bytes().as_ref(), collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_vault: InterfaceAccount<'info, TokenAccount>,

    // @note market creator can be mutated
    // @audit-note what if market creator is changed, are there any impacts?
    #[account(mut)]
    pub creator: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Supply<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"user_deposit", user.key().as_ref(), market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = user_deposit.bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_supply_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"user_deposit", user.key().as_ref(), market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = user_deposit.bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_supply_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Supply oracle account
    pub supply_oracle: AccountInfo<'info>,
    /// CHECK: Collateral oracle account
    pub collateral_oracle: AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Borrow<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"collateral_vault", market_id.to_le_bytes().as_ref(), collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"user_deposit", user.key().as_ref(), market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = user_deposit.bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_supply_account: InterfaceAccount<'info, TokenAccount>, // User's account to receive borrowed supply_mint
    #[account(mut)]
    pub user_collateral_account: InterfaceAccount<'info, TokenAccount>, // User's account to provide collateral_mint
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: Oracle account for collateral pricing
    pub collateral_oracle: AccountInfo<'info>,
    /// CHECK: Oracle account for borrow asset pricing
    pub borrow_oracle: AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct WithdrawCollateral<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"collateral_vault", market_id.to_le_bytes().as_ref(), collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"user_deposit", user.key().as_ref(), market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = user_deposit.bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_collateral_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Repay<'info> {
    #[account(
        mut,
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"user_deposit", user.key().as_ref(), market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = user_deposit.bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_supply_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Liquidate<'info> {
    #[account(
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"collateral_vault", market_id.to_le_bytes().as_ref(), collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_vault: InterfaceAccount<'info, TokenAccount>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub borrower_deposit: Account<'info, UserDeposit>,
    #[account(mut)]
    pub liquidator_supply_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub liquidator_collateral_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub liquidator: Signer<'info>,
    /// CHECK: Oracle account for pricing
    pub oracle: AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct FlashLoan<'info> {
    #[account(
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"supply_vault", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref()],
        bump
    )]
    pub supply_vault: InterfaceAccount<'info, TokenAccount>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user_supply_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct InitializeUserDeposit<'info> {
    /// CHECK: User deposit account to be created manually
    #[account(mut)]
    pub user_deposit: AccountInfo<'info>,
    #[account(
        seeds = [b"market", market_id.to_le_bytes().as_ref(), supply_mint.key().as_ref(), collateral_mint.key().as_ref()],
        bump = market.bump
    )]
    pub market: Account<'info, Market>,
    pub supply_mint: InterfaceAccount<'info, Mint>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseUserDeposit<'info> {
    #[account(
        mut,
        close = user,
    )]
    // @audit-note no check if the signer(user) closing the user_deposit account is the user who created that account
    pub user_deposit: Account<'info, UserDeposit>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseUninitializedAccount<'info> {
    /// CHECK: Target account to close
    #[account(mut)]
    pub target_account: AccountInfo<'info>,
    #[account(mut)]
    pub rent_receiver: Signer<'info>,
}

#[derive(Accounts)]
// @audit-note unauthorized market params update, missing check if authority is a market admin
pub struct UpdateMarketParams<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(source: Vec<u8>)]
pub struct CreateOracle<'info> {
    #[account(
        init,
        payer = authority,
        space = Oracle::space_for_source(source.len()),
        seeds = [b"oracle", mint.key().as_ref()],
        bump
    )]
    pub oracle: Account<'info, Oracle>,

    pub mint: Account<'info, anchor_spl::token::Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateOraclePrice<'info> {
    #[account(
        mut,
        seeds = [b"oracle", oracle.mint.as_ref()],
        bump = oracle.bump,
        has_one = authority
    )]
    pub oracle: Account<'info, Oracle>,

    pub authority: Signer<'info>,
}
