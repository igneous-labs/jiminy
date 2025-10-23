#![allow(unexpected_cfgs)]

use jiminy_cpi::{
    account::{Abr, AccountHandle},
    program_error::BuiltInProgramError,
    Cpi,
};
use jiminy_entrypoint::program_error::ProgramError;
use jiminy_system_prog_interface::{TransferIxAccs, TransferIxData};

pub const MAX_ACCS: usize = 3;

pub const MAX_CPI_ACCS: usize = 3;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    abr: &mut Abr,
    accounts: &[AccountHandle<'_>],
    data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let trf_amt = match data.split_first_chunk() {
        Some((trf_amt_bytes, _)) => u64::from_le_bytes(*trf_amt_bytes),
        _ => return Err(ProgramError::custom(1)),
    };

    let (sys_prog, transfer_accs) = match accounts.split_last_chunk() {
        Some((&[sys_prog], ta)) => (sys_prog, TransferIxAccs(*ta)),
        _ => {
            return Err(ProgramError::from_builtin(
                BuiltInProgramError::NotEnoughAccountKeys,
            ))
        }
    };

    let [from_lamports_bef, to_lamports_bef] =
        [transfer_accs.from(), transfer_accs.to()].map(|handle| abr.get(*handle).lamports());

    Cpi::<MAX_CPI_ACCS>::new().invoke_fwd_handle(
        abr,
        sys_prog,
        TransferIxData::new(trf_amt).as_buf(),
        transfer_accs.0,
    )?;

    if abr.get(*transfer_accs.from()).lamports() != from_lamports_bef - trf_amt {
        return Err(ProgramError::custom(2));
    }
    if abr.get(*transfer_accs.to()).lamports() != to_lamports_bef + trf_amt {
        return Err(ProgramError::custom(3));
    }

    Ok(())
}
