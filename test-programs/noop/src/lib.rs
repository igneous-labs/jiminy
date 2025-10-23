//! No-op program

#![allow(unexpected_cfgs)]

use jiminy_entrypoint::{
    account::{Abr, AccountHandle},
    program_error::ProgramError,
};

pub const MAX_ACCS: usize = 0;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    _abr: &mut Abr,
    _accounts: &[AccountHandle<'_>],
    _data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    Ok(())
}
