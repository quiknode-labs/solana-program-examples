use anchor_lang::prelude::*;

use crate::state::PowerStatus;

#[derive(Accounts)]
pub struct SetPowerStatus<'info> {
    #[account(mut)]
    pub power: Account<'info, PowerStatus>,
}

pub fn handler(ctx: Context<SetPowerStatus>, name: String) -> Result<()> {
    let power = &mut ctx.accounts.power;
    power.is_on = !power.is_on;

    msg!("{} is pulling the power switch!", &name);

    match power.is_on {
        true => msg!("The power is now on."),
        false => msg!("The power is now off!"),
    };

    Ok(())
}
