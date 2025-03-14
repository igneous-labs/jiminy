#![allow(unexpected_cfgs)]

use jiminy_cpi::invoke_signed;
use jiminy_entrypoint::{account::AccountHandle, program_error::ProgramError};
use jiminy_system_prog_interface::{transfer_ix, TransferAccounts};

pub const MAX_ACCS: usize = 3;

pub const MAX_CPI_ACCS: usize = 3;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    account_handles: &[AccountHandle],
    data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let Some(trf_amt_bytes): Option<&[u8; 8]> = data
        .get(..8)
        .map_or_else(|| None, |subslice| subslice.try_into().ok())
    else {
        return Err(ProgramError::Custom(1));
    };
    let trf_amt = u64::from_le_bytes(*trf_amt_bytes);

    // what the fuck changing from
    //
    // let [sys_prog, from, to] = core::array::from_fn(|i| account_handles[i]);
    //
    // to this cut program size in half from 9832 to 4656
    let Some([sys_prog, from, to]) = account_handles.get(..3) else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let [from_lamports_bef, to_lamports_bef] =
        [from, to].map(|handle| accounts.get(*handle).lamports());

    invoke_signed::<MAX_ACCS, MAX_CPI_ACCS>(
        accounts,
        transfer_ix(
            *sys_prog,
            TransferAccounts {
                from: *from,
                to: *to,
            },
            trf_amt,
        )
        .as_instr(),
        &[],
    )?;

    if accounts.get(*from).lamports() != from_lamports_bef - trf_amt {
        return Err(ProgramError::Custom(2));
    }
    if accounts.get(*to).lamports() != to_lamports_bef + trf_amt {
        return Err(ProgramError::Custom(3));
    }

    Ok(())
}
