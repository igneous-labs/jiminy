//! This program sets return data to the parameters (accounts, ix_data, prog_id)
//! passed to it in the following format:
//! - first part of buffer is accounts:
//!     - 35 bytes per account [..pubkey, is_executable, is_signer, is_writable]
//!     - up to first 8 accounts received
//! - second part is ix data:
//!     - up to 1024 - 32 - space taken up by first part bytes of ix data
//! - last 32 bytes is prog_id

#![allow(unexpected_cfgs)]

use std::{cmp::min, mem::MaybeUninit};

use jiminy_entrypoint::{
    account::{Abr, AccountHandle},
    program_error::ProgramError,
};
use jiminy_return_data::{set_return_data, ReturnData, MAX_RETURN_DATA};

/// Keep this low to test handling of discarded accounts
/// Also, 122 is max without running into stack limits
pub const MAX_ACCS: usize = 8;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    abr: &mut Abr,
    accounts: &[AccountHandle<'_>],
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let mut ret = [0u8; MAX_RETURN_DATA];
    let mut i = put_accounts(&mut ret, abr, accounts.iter().copied());

    let remaining = MAX_RETURN_DATA - i;
    let data_len_truncated = min(remaining.saturating_sub(32), data.len());
    ret[i..i + data_len_truncated].copy_from_slice(&data[..data_len_truncated]);
    i += data_len_truncated;

    ret[i..i + 32].copy_from_slice(prog_id);

    let ret = &ret[..i + 32];

    set_return_data(ret);

    let mut ret_data: MaybeUninit<ReturnData> = MaybeUninit::uninit();

    let Some(ret_data) = ReturnData::overwrite(&mut ret_data) else {
        return Err(ProgramError::custom(1));
    };
    if ret_data.program_id() != prog_id {
        return Err(ProgramError::custom(2));
    }
    if ret_data.data() != ret {
        return Err(ProgramError::custom(3));
    }

    Ok(())
}

// split separate fn with inline(never) to minimize stack usage
fn put_accounts<'a>(
    ret: &mut [u8],
    abr: &Abr,
    account_handles: impl IntoIterator<Item = AccountHandle<'a>>,
) -> usize {
    let mut i = 0;

    for h in account_handles {
        let acc = abr.get(h);

        ret[i..i + 32].copy_from_slice(acc.key());
        i += 32;

        for b in [acc.is_executable(), acc.is_signer(), acc.is_writable()] {
            ret[i] = if b { 1 } else { 0 };
            i += 1;
        }
    }

    i
}
