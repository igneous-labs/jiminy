//! This program calls Clock::get() and echoes the received data in return data

#![allow(unexpected_cfgs)]

use std::mem::MaybeUninit;

use jiminy_entrypoint::program_error::ProgramError;
use jiminy_return_data::set_return_data;
use jiminy_sysvar_clock::Clock;

pub const MAX_ACCS: usize = 0;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    _accounts: &mut Accounts,
    _data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let mut clock = MaybeUninit::uninit();
    let clock = Clock::sysvar_write_to(&mut clock)?;
    set_return_data(clock.as_account_data_arr());
    Ok(())
}
