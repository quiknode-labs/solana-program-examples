pub mod instructions;

use anchor_lang::prelude::*;

pub use instructions::*;

declare_id!("B56s2CGHWSG9HzzyYS5USQNYPJsB2teiUSKgG5CKpr2o");

#[program]
pub mod checking_account_program {
    use super::*;

    pub fn check_accounts(_ctx: Context<CheckingAccounts>) -> Result<()> {
        check_accounts::handler(_ctx)
    }
}
