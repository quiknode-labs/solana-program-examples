pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("AWYkDL963P4NXrb9kvik9JTRH7igkhJ4Xyd8t5YRv8ZN");

#[program]
pub mod anchor_test {
    use super::*;

    pub fn read_price(ctx: Context<ReadPrice>) -> Result<()> {
        read_price::handler(ctx)
    }
}
