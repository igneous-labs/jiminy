//! This program sets return data to the parameters (accounts, ix_data, prog_id)
//! passed to it in the following format:
//! - first part of buffer is accounts:
//!     - 35 bytes per account [..pubkey, is_executable, is_signer, is_writable]
//!     - up to first 8 accounts received
//! - second part is ix data:
//!     - up to 1024 - 32 - space taken up by first part bytes of ix data
//! - last 32 bytes is prog_id

#![allow(unexpected_cfgs)]

use std::cmp::min;

use jiminy_entrypoint::program_error::ProgramError;
use jiminy_return_data::{get_return_data, set_return_data, MAX_RETURN_DATA};

/// Keep this low to test handling of discarded accounts
/// Also, 122 is max without running into stack limits
pub const MAX_ACCS: usize = 8;

type Accounts<'a> = jiminy_entrypoint::account::Accounts<'a, MAX_ACCS>;
type AccountHandles<'a> = jiminy_entrypoint::account::AccountHandles<'a, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    account_handles: &AccountHandles,
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let mut ret = [0u8; MAX_RETURN_DATA];
    let mut i = put_accounts(&mut ret, accounts, account_handles);

    let remaining = MAX_RETURN_DATA - i;
    let data_len_truncated = min(remaining.saturating_sub(32), data.len());
    ret[i..i + data_len_truncated].copy_from_slice(&data[..data_len_truncated]);
    i += data_len_truncated;

    ret[i..i + 32].copy_from_slice(prog_id);

    set_return_data(&ret);

    let Some(ret_data) = get_return_data::<MAX_RETURN_DATA>() else {
        return Err(ProgramError::Custom(1));
    };
    if ret_data.program_id() != prog_id {
        return Err(ProgramError::Custom(2));
    }
    if ret_data.data() != ret {
        return Err(ProgramError::Custom(3));
    }

    Ok(())
}

// split separate fn with inline(never) to minimize stack usage
fn put_accounts(ret: &mut [u8], accounts: &Accounts, account_handles: &AccountHandles) -> usize {
    let mut i = 0;

    for h in account_handles {
        let acc = accounts.get(h);

        ret[i..i + 32].copy_from_slice(acc.key());
        i += 32;

        for b in [acc.is_executable(), acc.is_signer(), acc.is_writable()] {
            ret[i] = if b { 1 } else { 0 };
            i += 1;
        }
    }

    i
}
