use anchor_lang::prelude::*;

use crate::id;

#[derive(Accounts)]
pub struct TransferSolWithProgram<'info> {
    /// CHECK: Use owner constraint to check account is owned by our program
    #[account(
        mut,
        owner = id() // value of declare_id!()
    )]
    payer: UncheckedAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
}

// Directly modifying lamports is only possible if the program is the owner of the account
pub fn handler(
    ctx: Context<TransferSolWithProgram>,
    amount: u64,
) -> Result<()> {
    **ctx.accounts.payer.try_borrow_mut_lamports()? -= amount;
    **ctx.accounts.recipient.try_borrow_mut_lamports()? += amount;
    Ok(())
}
