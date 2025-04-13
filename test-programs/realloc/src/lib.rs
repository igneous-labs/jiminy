//! This program reallocs the single input account twice
//! to the 2 sizes specified by ix data,
//! zero initializing each time.

#![allow(unexpected_cfgs)]

use jiminy_entrypoint::program_error::ProgramError;

pub const MAX_ACCS: usize = 128;

type Accounts<'a> = jiminy_entrypoint::account::Accounts<'a, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let mut accounts_itr = accounts.iter();
    let [Some(acc)] = core::array::from_fn(|_| accounts_itr.next()) else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let [Some(r1), Some(r2)] = core::array::from_fn(|i| {
        data.get(i * 8..i * 8 + 8)
            .map(|slice| u64::from_le_bytes(*<&[u8; 8]>::try_from(slice).unwrap()) as usize)
    }) else {
        return Err(ProgramError::Custom(1));
    };

    let acc = accounts.get_mut(acc);
    acc.realloc(r1, true)?;
    acc.realloc(r2, true)?;

    Ok(())
}
