//! This program creates an account assigned to itself with the following ix data params
//! - size: u64
//! - lamports: Option<u64>, if omitted, uses `Rent::min_balance(size)`

#![allow(unexpected_cfgs)]

use jiminy_entrypoint::program_error::{BuiltInProgramError, ProgramError};
use jiminy_system_prog_interface::{
    create_account_ix, CreateAccountIxAccounts, CreateAccountIxData,
};
use jiminy_sysvar_rent::{sysvar::SimpleSysvar, Rent};

pub const MAX_ACCS: usize = 3;
pub const MAX_CPI_ACCS: usize = 3;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;
type Cpi = jiminy_cpi::Cpi<MAX_CPI_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let Some((accs, _rem)) = accounts.as_slice().split_first_chunk() else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::NotEnoughAccountKeys,
        ));
    };
    let [payer, acc, sys_prog] = *accs;

    // this is uber fukt but moving the ix_data processing blocks
    // down here instead of before the accounts processing part
    // results in a binary that is ~200 bytes smaller

    let Some((space, rem)) = data
        .split_first_chunk()
        .map(|(slice, rem)| (u64::from_le_bytes(*slice), rem))
    else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidArgument,
        ));
    };

    let lamports = match rem.split_first_chunk() {
        Some((s, _rem)) => u64::from_le_bytes(*s),
        None => {
            let rent = Rent::get()?;
            rent.min_balance_u64(space)
        }
    };

    Cpi::new().invoke_signed(
        accounts,
        create_account_ix(
            sys_prog,
            CreateAccountIxAccounts::memset(sys_prog)
                .with_funding(payer)
                .with_new(acc),
            &CreateAccountIxData::new(lamports, space as usize, prog_id),
        ),
        &[],
    )
}
