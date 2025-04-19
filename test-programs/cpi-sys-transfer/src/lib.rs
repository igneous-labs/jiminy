#![allow(unexpected_cfgs)]

use jiminy_cpi::{program_error::BuiltInProgramError, Cpi};
use jiminy_entrypoint::program_error::ProgramError;
use jiminy_system_prog_interface::{transfer_ix, TransferIxAccs, TransferIxData};

pub const MAX_ACCS: usize = 3;

pub const MAX_CPI_ACCS: usize = 3;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let Some(trf_amt_bytes): Option<&[u8; 8]> = data
        .get(..8)
        .map_or_else(|| None, |subslice| subslice.try_into().ok())
    else {
        return Err(ProgramError::custom(1));
    };
    let trf_amt = u64::from_le_bytes(*trf_amt_bytes);

    let [sys_prog, from, to] = accounts.as_slice() else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::NotEnoughAccountKeys,
        ));
    };
    let [sys_prog, from, to] = [sys_prog, from, to].map(|h| *h);

    let [from_lamports_bef, to_lamports_bef] =
        [from, to].map(|handle| accounts.get(handle).lamports());

    // use sys_prog as placeholder to avoid unsafe code
    let transfer_accounts = TransferIxAccs::memset(sys_prog).with_from(from).with_to(to);
    Cpi::<MAX_CPI_ACCS>::new().invoke_signed(
        accounts,
        transfer_ix(sys_prog, transfer_accounts, &TransferIxData::new(trf_amt)),
        &[],
    )?;

    if accounts.get(from).lamports() != from_lamports_bef - trf_amt {
        return Err(ProgramError::custom(2));
    }
    if accounts.get(to).lamports() != to_lamports_bef + trf_amt {
        return Err(ProgramError::custom(3));
    }

    Ok(())
}
