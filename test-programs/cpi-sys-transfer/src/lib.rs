#![allow(unexpected_cfgs)]

use jiminy_cpi::{program_error::BuiltInProgramError, Cpi};
use jiminy_entrypoint::program_error::ProgramError;
use jiminy_system_prog_interface::{TransferIxAccs, TransferIxData};

pub const MAX_ACCS: usize = 3;

pub const MAX_CPI_ACCS: usize = 3;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let trf_amt = match data.split_first_chunk() {
        Some((trf_amt_bytes, _)) => u64::from_le_bytes(*trf_amt_bytes),
        _ => return Err(ProgramError::custom(1)),
    };

    let (sys_prog, transfer_accs) = match accounts.as_slice().split_last_chunk() {
        Some((&[sys_prog], ta)) => (sys_prog, TransferIxAccs(*ta)),
        _ => {
            return Err(ProgramError::from_builtin(
                BuiltInProgramError::NotEnoughAccountKeys,
            ))
        }
    };

    let [from_lamports_bef, to_lamports_bef] =
        [transfer_accs.from(), transfer_accs.to()].map(|handle| accounts.get(*handle).lamports());

    let sys_prog_key = *accounts.get(sys_prog).key();
    Cpi::<MAX_CPI_ACCS>::new().invoke_signed(
        accounts,
        &sys_prog_key,
        TransferIxData::new(trf_amt).as_buf(),
        transfer_accs.into_account_handle_perms(),
        &[],
    )?;

    if accounts.get(*transfer_accs.from()).lamports() != from_lamports_bef - trf_amt {
        return Err(ProgramError::custom(2));
    }
    if accounts.get(*transfer_accs.to()).lamports() != to_lamports_bef + trf_amt {
        return Err(ProgramError::custom(3));
    }

    Ok(())
}
