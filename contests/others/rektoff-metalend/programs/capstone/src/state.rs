use anchor_lang::prelude::*;

/// Global protocol configuration and admin controls
#[account]
pub struct ProtocolState {
    pub admin: Pubkey,
    pub total_markets: u64,
    pub is_paused: bool,
    pub bump: u8,
}

impl ProtocolState {
    pub const SPACE: usize = 8 + 32 + 8 + 1 + 1; // discriminator + admin + total_markets + is_paused + bump
}

/// Individual lending markets with supply and collateral assets
#[account]
pub struct Market {
    pub market_id: u64,
    pub supply_mint: Pubkey,         // Asset that gets supplied and borrowed
    pub collateral_mint: Pubkey,     // Asset required as collateral to borrow
    pub market_admin: Pubkey,        // Admin/creator of this market
    pub total_supply_deposits: u128, // Total supply mint deposited for lending
    pub total_borrows: u128,         // Total supply mint borrowed
    pub total_collateral_deposits: u128, // Total collateral mint deposited
    pub total_ctoken_supply: u128,   // Total cTokens minted for this market
    pub collateral_factor: u64,      // Basis points (can stay u64)
    pub liquidation_threshold: u64,  // Basis points (can stay u64)
    pub last_update_slot: u64,
    pub cumulative_borrow_rate: u128, // Scaled by 10^9, needs u128 for calculations
    pub cumulative_supply_rate: u128, // Scaled by 10^9 for cToken interest, needs u128
    pub supply_oracle: Pubkey,        // Oracle for supply asset price
    pub collateral_oracle: Pubkey,    // Oracle for collateral asset price
    pub bump: u8,
    pub is_active: bool,
}

impl Market {
    pub fn space() -> usize {
        8 + // discriminator
        8 + // market_id
        32 + // supply_mint
        32 + // collateral_mint
        32 + // market_admin
        16 + // total_supply_deposits (u128)
        16 + // total_borrows (u128)
        16 + // total_collateral_deposits (u128)
        16 + // total_ctoken_supply (u128)
        8 + // collateral_factor
        8 + // liquidation_threshold
        8 + // last_update_slot
        16 + // cumulative_borrow_rate (u128)
        16 + // cumulative_supply_rate (u128)
        32 + // supply_oracle
        32 + // collateral_oracle
        1 + // bump
        1 // is_active
    }
}

/// Per-user account tracking supply deposits, collateral deposits, borrows, and cToken balances
#[account]
pub struct UserDeposit {
    pub user: Pubkey,
    pub market: Pubkey,
    pub supply_deposited: u128, // Amount of supply_mint deposited (for earning interest)
    pub collateral_deposited: u128, // Amount of collateral_mint deposited (as collateral)
    pub borrowed_amount: u128,  // Amount of supply_mint borrowed
    pub ctoken_balance: u128,   // cTokens from supply deposits
    pub last_update_slot: u64,
    pub bump: u8,
}

impl UserDeposit {
    pub const SPACE: usize = 8 + 32 + 32 + 16 + 16 + 16 + 16 + 8 + 1; // Updated for u128 fields
}

/// Oracle account for price feeds with proper validation
#[account]
pub struct Oracle {
    pub mint: Pubkey,      // The asset mint this oracle provides price for
    pub source: Vec<u8>,   // External data source integration (e.g., Pyth, Switchboard data)
    pub price: u128, // Current price with proper decimal scaling (u128 for large price calculations)
    pub decimals: u8, // Price decimal places
    pub valid_slot: u64, // Last slot when price was updated
    pub confidence: u128, // Price confidence interval (u128 for consistency)
    pub authority: Pubkey, // Authority that can update this oracle
    pub bump: u8,
}

impl Oracle {
    pub fn space_for_source(source_len: usize) -> usize {
        8 +                     // discriminator
        32 +                    // mint
        (4 + source_len) +      // source (Vec<u8>)
        16 +                    // price (u128)
        1 +                     // decimals
        8 +                     // valid_slot
        16 +                    // confidence (u128)
        32 +                    // authority
        1 // bump
    }

    /// Check if the oracle data is still valid (within acceptable staleness)
    pub fn is_valid(&self, current_slot: u64, max_staleness_slots: u64) -> bool {
        current_slot <= self.valid_slot + max_staleness_slots
    }
}
