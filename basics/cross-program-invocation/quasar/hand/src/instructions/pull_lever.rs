use quasar_lang::prelude::*;

/// Accounts for the hand program's pull_lever instruction.
/// The lever_program uses `Program<LeverProgram>` with a custom marker type
/// that implements `Id` — this lets Quasar verify the program address and
/// the executable flag during account parsing.
#[derive(Accounts)]
pub struct PullLever {
    #[account(mut)]
    pub power: UncheckedAccount,
    pub lever_program: Program<crate::LeverProgram>,
}

#[inline(always)]
pub fn handle_pull_lever(accounts: &PullLever, name: &str) -> Result<(), ProgramError> {
    log("Hand is pulling the lever!");

    // Build the switch_power instruction data for the lever program:
    //   [disc=1] [name: u32 len + bytes]
    // 128 bytes is enough for any reasonable name.
    let mut data = [0u8; 128];
    let name_bytes = name.as_bytes();
    let data_len = 1 + 4 + name_bytes.len();

    data[0] = 1;

    let len_bytes = (name_bytes.len() as u32).to_le_bytes();
    data[1] = len_bytes[0];
    data[2] = len_bytes[1];
    data[3] = len_bytes[2];
    data[4] = len_bytes[3];

    let mut i = 0;
    while i < name_bytes.len() {
        data[5 + i] = name_bytes[i];
        i += 1;
    }

    let mut cpi = DynCpiCall::<1, 128>::new(accounts.lever_program.address());
    cpi.push_account(accounts.power.to_account_view(), false, true)?;
    cpi.set_data(&data[..data_len])?;
    cpi.invoke()
}
