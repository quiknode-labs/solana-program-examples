use anchor_lang::prelude::*;

use crate::state::PriceUpdateV2;

#[derive(Accounts)]
pub struct ReadPrice<'info> {
    /// A PriceUpdateV2 account owned by the Pyth Receiver program.
    pub price_update: Account<'info, PriceUpdateV2>,
}

pub fn handler(ctx: Context<ReadPrice>) -> Result<()> {
    let price_update = &ctx.accounts.price_update;
    msg!("Price feed id: {:?}", price_update.price_message.feed_id);
    msg!("Price: {:?}", price_update.price_message.price);
    msg!("Confidence: {:?}", price_update.price_message.conf);
    msg!("Exponent: {:?}", price_update.price_message.exponent);
    msg!("Publish Time: {:?}", price_update.price_message.publish_time);
    Ok(())
}
