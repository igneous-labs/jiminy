//! No-op program

#![allow(unexpected_cfgs)]

use jiminy_entrypoint::program_error::ProgramError;

pub const MAX_ACCS: usize = 0;

type Accounts<'a> = jiminy_entrypoint::account::Accounts<'a, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    _accounts: &mut Accounts,
    _data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    Ok(())
}
