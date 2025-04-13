#![allow(unexpected_cfgs)]

use jiminy_cpi::Cpi;
use jiminy_entrypoint::program_error::ProgramError;
use jiminy_system_prog_interface::{transfer_ix, TransferAccounts};

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
        return Err(ProgramError::Custom(1));
    };
    let trf_amt = u64::from_le_bytes(*trf_amt_bytes);

    // what the fuck changing from
    //
    // let [sys_prog, from, to] = core::array::from_fn(|i| account_handles[i]);
    //
    // to this cut program size in half from 9832 to 4656
    let mut accounts_itr = accounts.iter();
    let [Some(sys_prog), Some(from), Some(to)] = core::array::from_fn(|_| accounts_itr.next())
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let [from_lamports_bef, to_lamports_bef] =
        [from, to].map(|handle| accounts.get(handle).lamports());

    Cpi::<MAX_CPI_ACCS>::new().invoke_signed(
        accounts,
        transfer_ix(sys_prog, TransferAccounts { from, to }, trf_amt).as_instr(),
        &[],
    )?;

    if accounts.get(from).lamports() != from_lamports_bef - trf_amt {
        return Err(ProgramError::Custom(2));
    }
    if accounts.get(to).lamports() != to_lamports_bef + trf_amt {
        return Err(ProgramError::Custom(3));
    }

    Ok(())
}
