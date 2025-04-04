#![allow(unexpected_cfgs)]

use jiminy_cpi::{invoke_signed, program_error::ProgramError};
use jiminy_entrypoint::account::AccountHandle;
use jiminy_pda::{
    create_program_address, try_find_program_address, PdaSeed, PdaSeedArr, PdaSigner,
};
use jiminy_system_prog_interface::{assign_ix, AssignAccounts};

pub const MAX_ACCS: usize = 2;
pub const MAX_CPI_ACCS: usize = 2;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    account_handles: &[AccountHandle],
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let Some([sys_prog, pda]) = account_handles.get(..2) else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let mut seeds = SeedsItr {
        data_remaining: data,
    }
    .collect::<Result<PdaSeedArr, ProgramError>>()?;

    // find
    let Some((pda_computed, bump)) = try_find_program_address(&seeds, prog_id) else {
        return Err(ProgramError::InvalidSeeds);
    };
    if pda_computed != *accounts.get(*pda).key() {
        return Err(ProgramError::Custom(1));
    }

    // create
    seeds
        .push(PdaSeed::new(core::slice::from_ref(&bump)))
        .map_err(|_full| ProgramError::InvalidArgument)?;
    let Some(pda_computed) = create_program_address(&seeds, prog_id) else {
        return Err(ProgramError::InvalidSeeds);
    };
    if pda_computed != *accounts.get(*pda).key() {
        return Err(ProgramError::Custom(2));
    }

    // assign pda to this prog
    invoke_signed::<MAX_ACCS, MAX_CPI_ACCS>(
        accounts,
        assign_ix(*sys_prog, AssignAccounts { assign: *pda }, *prog_id).as_instr(),
        &[PdaSigner::new(&seeds)],
    )?;

    if accounts.get(*pda).owner() != prog_id {
        return Err(ProgramError::Custom(3));
    }
    Ok(())
}

struct SeedsItr<'a> {
    data_remaining: &'a [u8],
}

impl<'a> Iterator for SeedsItr<'a> {
    type Item = Result<PdaSeed<'a>, ProgramError>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = *self.data_remaining.first()?;
        let end = 1 + usize::from(len);
        let Some(subslice) = self.data_remaining.get(1..end) else {
            return Some(Err(ProgramError::InvalidInstructionData));
        };
        let res = PdaSeed::new(subslice);
        self.data_remaining = &self.data_remaining[end..];
        Some(Ok(res))
    }
}
