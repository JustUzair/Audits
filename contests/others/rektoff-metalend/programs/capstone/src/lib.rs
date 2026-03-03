use anchor_lang::prelude::*;

declare_id!("AYye92emHVPgnxDHnTEkuuWVLUKF7JHKgWsXysZBZ3qe");

pub use contexts::*;
pub use errors::*;
pub use state::*;

pub mod contexts;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

pub use instructions::*;

#[program]
pub mod meta_lend {
    use super::*;

    /// Initialize the lending protocol
    pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
        instructions::initialize_protocol(ctx)
    }

    /// Create a new lending market for any SPL token
    pub fn create_market(
        ctx: Context<CreateMarket>,
        market_id: u64,
        collateral_factor: u64,
        liquidation_threshold: u64,
    ) -> Result<()> {
        instructions::create_market(ctx, market_id, collateral_factor, liquidation_threshold)
    }

    /// Supply tokens to earn interest (mint cTokens)
    pub fn supply(ctx: Context<Supply>, market_id: u64, amount: u64) -> Result<()> {
        instructions::supply(ctx, market_id, amount)
    }

    /// Withdraw supplied tokens (burn cTokens)
    pub fn withdraw(ctx: Context<Withdraw>, market_id: u64, ctoken_amount: u64) -> Result<()> {
        instructions::withdraw(ctx, market_id, ctoken_amount)
    }

    /// Borrow supply tokens by depositing collateral tokens
    pub fn borrow(
        ctx: Context<Borrow>,
        market_id: u64,
        collateral_amount: u64,
        borrow_amount: u64,
    ) -> Result<()> {
        instructions::borrow(ctx, market_id, collateral_amount, borrow_amount)
    }

    /// Withdraw collateral tokens (only allowed when no outstanding borrows)
    pub fn withdraw_collateral(
        ctx: Context<WithdrawCollateral>,
        market_id: u64,
        collateral_amount: u64,
    ) -> Result<()> {
        instructions::withdraw_collateral(ctx, market_id, collateral_amount)
    }

    /// Repay borrowed tokens
    pub fn repay(ctx: Context<Repay>, market_id: u64, amount: u64) -> Result<()> {
        instructions::repay(ctx, market_id, amount)
    }

    /// Liquidate undercollateralized positions
    pub fn liquidate(
        ctx: Context<Liquidate>,
        market_id: u64,
        liquidation_amount: u64,
    ) -> Result<()> {
        instructions::liquidate(ctx, market_id, liquidation_amount)
    }

    /// Flash loan functionality with external callback
    pub fn flash_loan(
        ctx: Context<FlashLoan>,
        market_id: u64,
        amount: u64,
        callback_data: Vec<u8>,
    ) -> Result<()> {
        instructions::flash_loan(ctx, market_id, amount, callback_data)
    }

    /// Initialize user deposit account
    pub fn initialize_user_deposit(
        ctx: Context<InitializeUserDeposit>,
        market_id: u64,
    ) -> Result<()> {
        instructions::initialize_user_deposit(ctx, market_id)
    }

    /// Close user deposit account
    pub fn close_user_deposit(ctx: Context<CloseUserDeposit>) -> Result<()> {
        instructions::close_user_deposit(ctx)
    }

    /// Update market parameters
    pub fn update_market_params(
        ctx: Context<UpdateMarketParams>,
        new_collateral_factor: u64,
        new_liquidation_threshold: u64,
    ) -> Result<()> {
        instructions::update_market_params(ctx, new_collateral_factor, new_liquidation_threshold)
    }

    /// Create oracle (simplified for demo)
    pub fn create_oracle(
        ctx: Context<CreateOracle>,
        source: Vec<u8>,
        initial_price: u64,
        decimals: u8,
    ) -> Result<()> {
        instructions::create_oracle(ctx, source, initial_price, decimals)
    }

    /// Update oracle price
    pub fn update_oracle_price(ctx: Context<UpdateOraclePrice>, new_price: u64) -> Result<()> {
        instructions::update_oracle_price(ctx, new_price)
    }
}
