use anchor_lang::prelude::*;

// automatically generate module using program idl found in ./idls
declare_program!(lever);
use lever::accounts::PowerStatus;
use lever::cpi::accounts::SwitchPower;
use lever::cpi::switch_power;
use lever::program::Lever;

#[derive(Accounts)]
pub struct PullLever<'info> {
    #[account(mut)]
    pub power: Account<'info, PowerStatus>,
    pub lever_program: Program<'info, Lever>,
}

pub fn handler(ctx: Context<PullLever>, name: String) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        ctx.accounts.lever_program.key(),
        SwitchPower {
            power: ctx.accounts.power.to_account_info(),
        },
    );
    switch_power(cpi_ctx, name)?;
    Ok(())
}
