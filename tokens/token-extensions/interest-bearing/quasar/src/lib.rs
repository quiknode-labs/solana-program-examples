#![no_std]

use quasar_lang::sysvars::Sysvar;
use quasar_lang::{
    cpi::{CpiCall, InstructionAccount},
    prelude::*,
};

#[cfg(test)]
mod tests;

declare_id!("22222222222222222222222222222222222222222222");

pub struct Token2022Program;
impl Id for Token2022Program {
    const ID: Address = Address::new_from_array([
        6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218,
        182, 26, 252, 77, 131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
    ]);
}

/// Creates a mint with the InterestBearingConfig extension and allows
/// updating the interest rate.
#[program]
mod quasar_interest_bearing {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn initialize(ctx: Ctx<Initialize>, rate: i16) -> Result<(), ProgramError> {
        ctx.accounts.initialize(rate)
    }

    #[instruction(discriminator = 1)]
    pub fn update_rate(ctx: Ctx<UpdateRate>, rate: i16) -> Result<(), ProgramError> {
        ctx.accounts.update_rate(rate)
    }
}

#[derive(Accounts)]
pub struct Initialize {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut)]
    pub mint_account: Signer,
    pub token_program: Program<Token2022Program>,
    pub system_program: Program<System>,
}

impl Initialize {
    #[inline(always)]
    pub fn initialize(&mut self, rate: i16) -> Result<(), ProgramError> {
        // 165 (base) + 1 (account type) + 4 (TLV header) + 52 (InterestBearingConfig data) = 222 bytes
        let mint_size: u64 = 222;
        let lamports = Rent::get()?.try_minimum_balance(mint_size as usize)?;

        self.system_program
            .create_account(
                &self.payer,
                &self.mint_account,
                lamports,
                mint_size,
                self.token_program.to_account_view().address(),
            )
            .invoke()?;

        // InterestBearingMintInitialize: opcode 33, sub-opcode 0
        // Data: [33, 0, rate_authority (32 bytes), rate (i16 LE)]
        let mut ext_data = [0u8; 36];
        ext_data[0] = 33;
        ext_data[1] = 0; // Initialize sub-opcode
        ext_data[2..34].copy_from_slice(self.payer.to_account_view().address().as_ref());
        ext_data[34..36].copy_from_slice(&rate.to_le_bytes());

        CpiCall::new(
            self.token_program.to_account_view().address(),
            [InstructionAccount::writable(
                self.mint_account.to_account_view().address(),
            )],
            [self.mint_account.to_account_view()],
            ext_data,
        )
        .invoke()?;

        // InitializeMint2
        let mut mint_data = [0u8; 67];
        mint_data[0] = 20;
        mint_data[1] = 2;
        mint_data[2..34].copy_from_slice(self.payer.to_account_view().address().as_ref());
        mint_data[34] = 1;
        mint_data[35..67].copy_from_slice(self.payer.to_account_view().address().as_ref());

        CpiCall::new(
            self.token_program.to_account_view().address(),
            [InstructionAccount::writable(
                self.mint_account.to_account_view().address(),
            )],
            [self.mint_account.to_account_view()],
            mint_data,
        )
        .invoke()
    }
}

#[derive(Accounts)]
pub struct UpdateRate {
    #[account(mut)]
    pub authority: Signer,
    #[account(mut)]
    pub mint_account: UncheckedAccount,
    pub token_program: Program<Token2022Program>,
}

impl UpdateRate {
    #[inline(always)]
    pub fn update_rate(&mut self, rate: i16) -> Result<(), ProgramError> {
        // InterestBearingMintUpdateRate: opcode 33, sub-opcode 1, rate (i16 LE)
        let mut data = [0u8; 4];
        data[0] = 33;
        data[1] = 1;
        data[2..4].copy_from_slice(&rate.to_le_bytes());

        CpiCall::new(
            self.token_program.to_account_view().address(),
            [
                InstructionAccount::writable(self.mint_account.to_account_view().address()),
                InstructionAccount::readonly_signer(self.authority.to_account_view().address()),
            ],
            [
                self.mint_account.to_account_view(),
                self.authority.to_account_view(),
            ],
            data,
        )
        .invoke()
    }
}
