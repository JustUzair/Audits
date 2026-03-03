use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Unauthorized operation")]
    Unauthorized,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Position is healthy - cannot liquidate")]
    PositionHealthy,
    #[msg("Excessive liquidation amount")]
    ExcessiveLiquidation,
    #[msg("Flash loan not repaid")]
    FlashLoanNotRepaid,
    #[msg("Account has deposits")]
    HasDeposits,
    #[msg("Account has borrows")]
    HasBorrows,
    #[msg("No lamports available to steal")]
    NoLamportsToSteal,
    #[msg("Account is already initialized")]
    AccountAlreadyInitialized,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Market not found")]
    MarketNotFound,
    #[msg("Market is paused")]
    MarketPaused,
    #[msg("Invalid oracle data")]
    InvalidOracleData,
    #[msg("Market is not active")]
    MarketNotActive,
    #[msg("Invalid market state")]
    InvalidMarketState,
    #[msg("User deposit account already exists")]
    UserDepositAlreadyExists,
    #[msg("Invalid PDA")]
    InvalidPDA,
}
