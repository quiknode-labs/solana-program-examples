pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("4tvjFDZybFML3tN5uyRoHPoxUTitTwGxjD7KfKEmX7Vg");

#[program]
pub mod hand {
    use super::*;

    pub fn pull_lever(ctx: Context<PullLever>, name: String) -> Result<()> {
        pull_lever::handler(ctx, name)
    }
}
