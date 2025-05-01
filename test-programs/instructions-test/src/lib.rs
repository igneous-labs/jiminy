//! This program deserializes the Instructions syvar piecewise and
//! puts it into return data

#![allow(unexpected_cfgs)]

use std::mem::MaybeUninit;

use jiminy_entrypoint::program_error::ProgramError;
use jiminy_return_data::set_return_data;
use jiminy_sysvar_instructions::Instructions;

pub const MAX_ACCS: usize = 0;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    _accounts: &mut Accounts,
    _data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let mut n_ixs = MaybeUninit::uninit();
    let n_ixs = Instructions::load_n_ixs_to(&mut n_ixs)?;
    set_return_data(&n_ixs.to_le_bytes());
    Ok(())
}
