//! This program closes an account owned by it.
//!
//! Note that this program cannot work on any actual solana network, because there is no way for it
//! to create the program owned account in the first place if it doesnt already exist.

#![allow(unexpected_cfgs)]

use jiminy_entrypoint::{
    account::{Abr, AccountHandle},
    program_error::{ProgramError, NOT_ENOUGH_ACCOUNT_KEYS},
};

pub const MAX_ACCS: usize = 2;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    abr: &mut Abr,
    accounts: &[AccountHandle],
    _data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let [close, refund_rent_to] = *accounts else {
        return Err(NOT_ENOUGH_ACCOUNT_KEYS.into());
    };
    abr.close(close, refund_rent_to)
}
