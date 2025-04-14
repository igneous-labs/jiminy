#![allow(unexpected_cfgs)]

use jiminy_cpi::{
    program_error::{BuiltInProgramError, ProgramError},
    Cpi,
};
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
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let mut accounts_itr = accounts.iter();
    let [Some(sys_prog), Some(pda)] = core::array::from_fn(|_| accounts_itr.next()) else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::NotEnoughAccountKeys,
        ));
    };
    let mut seeds = SeedsItr {
        data_remaining: data,
    }
    .collect::<Result<PdaSeedArr, ProgramError>>()?;

    // find
    let Some((pda_computed, bump)) = try_find_program_address(&seeds, prog_id) else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidSeeds,
        ));
    };
    if pda_computed != *accounts.get(pda).key() {
        return Err(ProgramError::custom(1));
    }

    // create
    seeds
        .push(PdaSeed::new(core::slice::from_ref(&bump)))
        .map_err(|_full| ProgramError::from_builtin(BuiltInProgramError::InvalidArgument))?;
    let Some(pda_computed) = create_program_address(&seeds, prog_id) else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidSeeds,
        ));
    };
    if pda_computed != *accounts.get(pda).key() {
        return Err(ProgramError::custom(2));
    }

    // assign pda to this prog
    Cpi::<MAX_CPI_ACCS>::new().invoke_signed(
        accounts,
        assign_ix(sys_prog, AssignAccounts { assign: pda }, *prog_id).as_instr(),
        &[PdaSigner::new(&seeds)],
    )?;

    if accounts.get(pda).owner() != prog_id {
        return Err(ProgramError::custom(3));
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
            return Some(Err(ProgramError::from_builtin(
                BuiltInProgramError::InvalidInstructionData,
            )));
        };
        let res = PdaSeed::new(subslice);
        self.data_remaining = &self.data_remaining[end..];
        Some(Ok(res))
    }
}
