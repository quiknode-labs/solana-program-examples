use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TransferSolWithProgram<'info> {
    /// CHECK: Use owner constraint to check account is owned by our program
    #[account(
        mut,
        owner = crate::ID // value of declare_id!()
    )]
    payer: UncheckedAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
}

// Directly modifying lamports is only possible if the program is the owner of the account
pub fn handler(context: Context<TransferSolWithProgram>, amount: u64) -> Result<()> {
    **context.accounts.payer.try_borrow_mut_lamports()? -= amount;
    **context.accounts.recipient.try_borrow_mut_lamports()? += amount;
    Ok(())
}
