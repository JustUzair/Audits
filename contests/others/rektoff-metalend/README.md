# MetaLend - Solana Lending Protocol

MetaLend is a decentralized lending and borrowing protocol designed for educational purposes. This protocol demonstrates the core business logic and requirements for a modern DeFi lending platform built on Solana.

**⚠️ IMPORTANT NOTE**: Any code sections marked with `CAPSTONE_SAFE` are considered correct implementations and should not be flagged as issues. These sections include simplified oracle updates and mock interest calculations intended for educational demonstration.

## 🏢 Business Requirements Overview

### Core Business Model
MetaLend operates as a **dual-asset lending protocol** where users can:
- **Supply assets** to earn interest through appreciating cTokens (compound tokens)
- **Deposit collateral** to borrow different assets against their deposits
- **Participate in liquidations** to maintain protocol solvency
- **Access flash loans** for advanced trading strategies

### Market Structure Requirements
Each lending market operates with two distinct asset types:
- **Supply Asset**: The token that gets lent out and borrowed (e.g., USDC, SOL)
- **Collateral Asset**: The token required as collateral to secure borrowings (e.g., BTC, ETH)

This dual-asset design allows for more flexible risk management and enables cross-asset borrowing scenarios.

## 🏗️ Technical Architecture

### Account Structure Requirements

#### ProtocolState Account
- Serves as the global registry for protocol configuration
- Maintains admin authority and total market count
- Controls protocol-wide pause functionality

#### Market Account
Each market requires the following data structure:
- **Asset Configuration**: Supply mint, collateral mint, and associated vaults
- **Financial Metrics**: Total supply deposits, collateral deposits, and outstanding borrows
- **cToken Management**: Total cToken supply and exchange rate calculations
- **Risk Parameters**: Collateral factor and liquidation threshold (in basis points)
- **Oracle Integration**: Price feed references for both supply and collateral assets
- **Interest Calculations**: Cumulative rates and last update tracking

#### UserDeposit Account
Per-user, per-market tracking account containing:
- Supply deposits (earning interest via cTokens)
- Collateral deposits (securing borrowing capacity)
- Outstanding borrowed amounts
- cToken balance tracking

### Business Process Flows

#### Market Creation Process
1. **Market Initialization**: Deploy new lending market with specified asset pairs
2. **Risk Parameter Setting**: Configure collateral factor and liquidation thresholds

#### Supply and Interest Earning Process
1. **Asset Deposit**: Users deposit supply assets to earn interest
2. **cToken Minting**: Protocol mints cTokens representing user's share of the pool
3. **Interest Accrual**: cTokens appreciate in value as borrowers pay interest

#### Collateralized Borrowing Process
1. **Collateral Deposit**: Users deposit collateral assets to secure borrowing capacity
2. **Borrowing Capacity Calculation**: Based on collateral value and configured collateral factor
3. **Asset Borrowing**: Users can borrow supply assets up to their capacity limit
4. **Ongoing Monitoring**: Position health tracked for liquidation eligibility

#### Liquidation Mechanism
- **Health Monitoring**: Continuous tracking of collateralization ratios
- **Liquidation Triggering**: When positions fall below liquidation threshold we should liquidate the position enough to bring it back to a healthy state plus a penalty of 10%
- **Liquidator Incentives**: Bonus rewards for maintaining protocol solvency (10% bonus)

### Oracles

MetaLend uses on-chain price oracles to determine the value of both supply and collateral assets. All oracles are controlled by the MetaLend, if needed, contact the admin to add new oracles.


## 🚀 Getting Started

### Prerequisites
- [Rust](https://rustup.rs/)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)  
- [Anchor Framework](https://www.anchor-lang.com/docs/installation)
- [Node.js](https://nodejs.org/) and [Yarn](https://yarnpkg.com/)

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd meta-lend
   ```

2. **Install dependencies**
   ```bash
   yarn install
   ```

3. **Build the program**
   ```bash
   anchor build
   ```

4. **Deploy to localnet**
   ```bash
   # Start local validator
   solana-test-validator
   
   # Deploy program
   anchor deploy
   ```

5. **Run tests**
   ```bash
   anchor test
   ```

## 📋 Business Logic Implementation

### Market Creation Business Rules

Markets are created with the following business parameters:

- **Market ID**: Unique identifier for the lending market
- **Collateral Factor**: Maximum borrowing power as percentage of collateral value (e.g., 80%)
- **Liquidation Threshold**: Health ratio below which positions become liquidatable (e.g., 85%)

```typescript
// Example: Create a USDC/BTC lending market
// Users can deposit USDC to earn interest, deposit BTC as collateral to borrow USDC
await program.methods
  .createMarket(
    new anchor.BN(1),        // market_id
    new anchor.BN(8000),     // collateral_factor (80% - can borrow up to 80% of collateral value)
    new anchor.BN(8500)      // liquidation_threshold (85% - liquidatable when below 85% health)
  )
  .accounts({
    market: marketAccount,
    protocolState,
    supplyMint: usdcMint,      // Asset that gets supplied/borrowed
    collateralMint: btcMint,   // Asset required as collateral
    // ... other accounts
  })
  .rpc();
```

### Supply Operations Business Logic

Users deposit supply assets to earn interest through cToken appreciation:

```typescript
// Supply 1000 USDC to earn interest
// User receives cTokens that appreciate over time
await program.methods
  .supply(new anchor.BN(1), new anchor.BN(1000 * 1e6))
  .accounts({
    market: usdcBtcMarket,
    supplyVault: usdcVault,        // Vault holding the supply assets
    userDeposit: userDepositAccount,
    userSupplyAccount: userUsdcAccount,
    user: user.publicKey,
  })
  .rpc();
```

### Borrowing Operations Business Logic

Users must deposit collateral before borrowing, with capacity calculated via oracle prices:

```typescript  
// 1. Deposit 1 BTC as collateral, then borrow 500 USDC
// Assuming BTC = $50,000, user can borrow up to $40,000 worth (80% collateral factor)
await program.methods
  .borrow(
    new anchor.BN(1),                    // market_id
    new anchor.BN(1 * 1e8),             // collateral_amount (1 BTC)
    new anchor.BN(500 * 1e6)            // borrow_amount (500 USDC)
  )
  .accounts({
    market: usdcBtcMarket,
    supplyVault: usdcVault,              // Vault to withdraw borrowed USDC from
    collateralVault: btcVault,           // Vault to deposit BTC collateral to
    userDeposit: userDepositAccount,
    userSupplyAccount: userUsdcAccount,   // User's USDC account (receives borrowed funds)
    userCollateralAccount: userBtcAccount, // User's BTC account (source of collateral)
    supplyOracle: usdcOracle,            // Price feed for USDC
    collateralOracle: btcOracle,         // Price feed for BTC
    user: user.publicKey,
  })
  .rpc();
```

### Collateral Withdrawal Business Logic

Once a borrower has repaid all outstanding debt, they can reclaim their posted collateral through the withdrawal flow implemented in `withdraw_collateral`@programs/capstone/src/instructions/borrow.rs#149-214:

```typescript
// Withdraw 0.25 BTC of collateral after all loans are cleared
await program.methods
  .withdrawCollateral(
    new anchor.BN(1),            // market_id
    new anchor.BN(0.25 * 1e8)    // collateral_amount (0.25 BTC)
  )
  .accounts({
    market: usdcBtcMarket,
    collateralVault: btcVault,          // Vault currently holding the BTC collateral
    userDeposit: userDepositAccount,
    userCollateralAccount: userBtcAccount, // User receives their BTC back here
    supplyMint: usdcMint,
    collateralMint: btcMint,
    user: user.publicKey,
  })
  .rpc();
```

Key enforcement steps:
1. **Outstanding Borrow Check** — The instruction rejects withdrawals while `borrowed_amount` > 0, ensuring collateral always backs active debt.
2. **Balance Validation** — Confirms the user has enough collateral deposited before releasing funds.
3. **Program-Derived Authority** — Uses the market PDA as signer so only protocol-controlled vaults can transfer collateral.
4. **State Accounting** — Updates both the `UserDeposit` and `Market` totals to keep solvency metrics accurate.

## ⚙️ Business Configuration Requirements

### Interest Rate Business Model

The protocol implements a simplified interest mechanism for educational purposes:
- **Supply Interest**: cTokens appreciate over time as borrowers pay interest
- **Interest Accrual**: Mock interest calculation provides steady appreciation
- **Exchange Rate**: cToken value increases relative to underlying assets

### Risk Management Parameters

Each market requires configuration of key business risk metrics:

- **Collateral Factor** (basis points): Maximum borrowing capacity relative to collateral value
  - Example: 8000 basis points = 80% = Can borrow up to 80% of collateral value
  
- **Liquidation Threshold** (basis points): Health ratio triggering liquidation eligibility
  - Example: 8500 basis points = 85% = Positions liquidatable when health drops below 85%

### Oracle Price Feed Requirements

Markets depend on oracle integration for business operations:
- **Collateral Valuation**: Determine borrowing capacity based on current collateral prices
- **Liquidation Monitoring**: Track position health using real-time price data
- **Cross-Asset Calculations**: Convert between different asset values for risk assessment

**Note**: Oracle updates in this educational implementation are marked `CAPSTONE_SAFE` and represent simplified price feed functionality for demonstration purposes.

## 🧪 Protocol Testing Requirements

The test suite validates core business functionality:

- ✅ Protocol initialization and governance
- ✅ Market creation with dual-asset configuration
- ✅ Supply operations and cToken mechanics  
- ✅ Collateral deposit and borrowing flows
- ✅ Interest accrual and exchange rate calculations
- ✅ Liquidation business logic
- ✅ Flash loan capabilities
- ✅ User account management

Run the complete test suite:
```bash
anchor test --skip-build
```

Run specific test files:
```bash  
anchor test --skip-build tests/meta-lend.ts
```

## CAPSTONE
Consider the exchange rate calculation in `calculate_exchange_rate` is correct and don't try to overwork it into issues.
Try to log as many issues as possible, what you can think of. Be true with yourself and try to not use LLMs, the real blackhats will outsmart you.



## ⚠️ Educational Disclaimer

This software is designed for educational purposes to demonstrate DeFi lending protocol concepts and Solana development patterns. It is not intended for production use with real funds.