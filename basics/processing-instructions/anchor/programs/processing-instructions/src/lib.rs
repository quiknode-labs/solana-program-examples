pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("ErP5EBTkp343iNqC9HP5u7Eh8dyTn5tvKzSiaUpiKgHK");

#[program]
pub mod processing_instructions {
    use super::*;

    pub fn go_to_park(_ctx: Context<Park>, name: String, height: u32) -> Result<()> {
        go_to_park::handler(_ctx, name, height)
    }
}
