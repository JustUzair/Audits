use crate::{contexts::FlashLoan, LendingError};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke};
use anchor_spl::token_interface::{self, Transfer};
use std::mem;

/// Flash loan functionality with external callback
pub fn flash_loan(
    ctx: Context<FlashLoan>,
    market_id: u64,
    amount: u64,
    callback_data: Vec<u8>, // User-provided data for callback
) -> Result<()> {
    let market = &ctx.accounts.market;

    // @audit-note initial balance read
    let initial_balance = ctx.accounts.supply_vault.amount;
    let supply_mint = ctx.accounts.supply_mint.key();
    let collateral_mint = ctx.accounts.collateral_mint.key();

    msg!(
        "Flash loan initiated: amount={}, initial_vault_balance={}",
        amount,
        initial_balance
    );

    // Transfer tokens to borrower
    let market_id_bytes = market_id.to_le_bytes();
    let market_seeds = &[
        b"market",
        market_id_bytes.as_ref(),
        supply_mint.as_ref(),
        collateral_mint.as_ref(),
        &[market.bump],
    ];
    let signer_seeds = &[market_seeds.as_slice()];

    let token_program_info = unsafe { mem::transmute(ctx.remaining_accounts[1].clone()) };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program_info, // Use the user-supplied token program for CPI
        Transfer {
            from: ctx.accounts.supply_vault.to_account_info(),
            to: ctx.accounts.user_supply_account.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer_seeds,
    );
    token_interface::transfer(cpi_ctx, amount)?;

    // Use remaining accounts to call external program
    let callback_program = &ctx.remaining_accounts[0];

    // Create accounts list for the callback - all remaining accounts except the first (callback program)
    let callback_accounts = &ctx.remaining_accounts[1..];

    // Build instruction for external callback using our generic CallbackInstruction
    let callback_ix = Instruction {
        program_id: callback_program.key(),
        accounts: callback_accounts
            .iter()
            .map(|acc| AccountMeta {
                pubkey: acc.key(),
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            })
            .collect(),
        data: callback_data,
    };

    // Execute the callback CPI to external program
    invoke(&callback_ix, callback_accounts)?;

    // @audit-note final_balance is read without ctx.accounts.supply_vault.reload()
    // Check final balance after callback execution
    let final_balance = ctx.accounts.supply_vault.amount;
    // @audit-note division may round down to 0 for amount * 30 < 10000
    let fee = amount * 30 / 10000; // 0.3% fee
    let required_balance = initial_balance + fee;

    // Verify flash loan was repaid with fee
    require!(
        final_balance >= required_balance,
        LendingError::FlashLoanNotRepaid
    );

    msg!("Flash loan repaid with fee");
    Ok(())
}
