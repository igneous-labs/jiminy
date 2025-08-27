#![allow(unexpected_cfgs)]

use std::mem::MaybeUninit;

use jiminy_cpi::program_error::{BuiltInProgramError, ProgramError};
use jiminy_pda::{
    create_program_address_to, create_raw_program_address_to, try_find_program_address_to, PdaSeed,
    PdaSeedArr, PdaSigner,
};
use jiminy_system_prog_interface::{AssignIxAccs, AssignIxData};

pub const MAX_ACCS: usize = 2;
pub const MAX_CPI_ACCS: usize = 2;

type Accounts<'account> = jiminy_entrypoint::account::Accounts<'account, MAX_ACCS>;
type Cpi = jiminy_cpi::Cpi<MAX_CPI_ACCS>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let [sys_prog, pda] = *accounts.as_slice() else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::NotEnoughAccountKeys,
        ));
    };

    let mut seeds = SeedsItr {
        data_remaining: data,
    }
    .collect::<Result<PdaSeedArr, ProgramError>>()?;

    // find
    let mut pda_computed_dst = MaybeUninit::uninit();
    let mut bump = MaybeUninit::uninit();
    let Some((pda_computed, bump)) =
        try_find_program_address_to(&seeds, prog_id, &mut pda_computed_dst, &mut bump)
    else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidSeeds,
        ));
    };
    if pda_computed != accounts.get(pda).key() {
        return Err(ProgramError::custom(1));
    }

    // create
    seeds
        .push(PdaSeed::new(core::slice::from_ref(bump)))
        .map_err(|_full| ProgramError::from_builtin(BuiltInProgramError::InvalidArgument))?;
    let Some(pda_computed) = create_program_address_to(&seeds, prog_id, &mut pda_computed_dst)
    else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidSeeds,
        ));
    };
    if pda_computed != accounts.get(pda).key() {
        return Err(ProgramError::custom(2));
    }

    // create raw
    seeds
        .for_create_raw(prog_id)
        .map_err(|_full| ProgramError::from_builtin(BuiltInProgramError::InvalidArgument))?;
    let Some(pda_computed_raw) = create_raw_program_address_to(&seeds, &mut pda_computed_dst)
    else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidSeeds,
        ));
    };
    if pda_computed_raw != accounts.get(pda).key() {
        return Err(ProgramError::custom(3));
    }

    // use sys_prog as placeholder to avoid unsafe code
    let assign_accounts = AssignIxAccs::memset(sys_prog).with_assign(pda);
    // assign pda to this prog
    Cpi::new().invoke_signed_handle(
        accounts,
        sys_prog,
        AssignIxData::new(prog_id).as_buf(),
        assign_accounts.into_account_handle_perms(),
        // last 2 item on `seeds` are program ID and PDA_MARKER, so omit those
        &[PdaSigner::new(seeds.split_last_chunk::<2>().unwrap().0)],
    )?;

    if accounts.get(pda).owner() != prog_id {
        return Err(ProgramError::custom(4));
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
