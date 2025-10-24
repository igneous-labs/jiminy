//! This program accepts the Instructions syvar as its only account input,
//! and accepts the following input data:
//!
//! - u16: total_ixs_len
//! - u16: instruction_idx
//! - u16: account_idx
//! - `[u8; 32]`: pubkey
//! - u8: is_writable
//! - u8: is_signer
//!
//! It then verifies the total_ixs_len against that returned by the instructions sysvar,
//! and that the account at `instructions[instruction_idx][account_idx]` has the same
//! `is_writable` and `is_signer` as the args

#![allow(unexpected_cfgs)]

use std::ptr;

use jiminy_entrypoint::{
    account::{Abr, AccountHandle},
    program_error::{BuiltInProgramError, ProgramError},
};
use jiminy_sysvar_instructions::Instructions;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct IxArgs {
    pub ixs_len: u16,
    pub curr_idx: u16,
    pub ix_idx: u16,
    pub ix_data_len: u16,
    pub acc_idx: u16,
    pub pubkey: [u8; 32],
    pub is_writable: u8,
    pub is_signer: u8,
}

impl IxArgs {
    #[inline]
    pub const fn as_buf(&self) -> &[u8; core::mem::size_of::<Self>()] {
        unsafe { &*ptr::from_ref(self).cast() }
    }
}

const MAX_ACCS: usize = 1;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    abr: &mut Abr,
    accounts: &[AccountHandle<'_>],
    data: &[u8],
    _prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    let [ixs] = *accounts else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::NotEnoughAccountKeys,
        ));
    };

    let Some(ixs) = Instructions::try_from_account(abr.get(ixs)) else {
        return Err(ProgramError::from_builtin(
            BuiltInProgramError::InvalidAccountData,
        ));
    };

    // safety:
    // - instruction data is guaranteed to be 8-byte aligned
    // - IxArgs is repr(C)
    // - IxArgs has no padding
    let IxArgs {
        ixs_len,
        ix_idx,
        ix_data_len,
        acc_idx,
        curr_idx,
        pubkey,
        is_writable,
        is_signer,
    } = unsafe { &*ptr::from_ref(data).cast() };

    if ixs.len_u16() != ixs_len {
        return Err(ProgramError::custom(0));
    }

    let Some(ix) = ixs.iter().nth(usize::from(*ix_idx)) else {
        return Err(ProgramError::custom(1));
    };

    if ix.data().len() != usize::from(*ix_data_len) {
        return Err(ProgramError::custom(2));
    }

    let Some(acc) = ix.accounts().get(usize::from(*acc_idx)) else {
        return Err(ProgramError::custom(3));
    };
    if acc.key() != pubkey {
        return Err(ProgramError::custom(4));
    }

    let flags = acc.flags();
    let [is_signer, is_writable] = [is_signer, is_writable].map(|b| *b != 0);
    if flags.is_signer() != is_signer {
        return Err(ProgramError::custom(5));
    }
    if flags.is_writable() != is_writable {
        return Err(ProgramError::custom(6));
    }

    if *curr_idx != ixs.current_idx_u16() {
        return Err(ProgramError::custom(7));
    }

    Ok(())
}
