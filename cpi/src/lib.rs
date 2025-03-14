#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

//! TODO: turbofish on all invoke_signed_*() calls is currently a bit annoying because
//! const generics cant be inferred yet.
//!
//! All the invocation functions take `&mut Accounts` as param
//! because we must have exclusive access to accounts since CPI may mutate accounts

use core::mem::MaybeUninit;

// Re-exports
pub mod account {
    pub use jiminy_account::*;
}
use account::*;
pub mod program_error {
    pub use jiminy_program_error::*;
}
use program_error::*;
pub mod pda {
    pub use jiminy_pda::*;
}
use pda::*;

mod cpi_account;
mod cpi_account_meta;
mod instruction;

pub use cpi_account::*;
pub use cpi_account_meta::*;
pub use instruction::*;

pub(crate) const ONE_KB: usize = 1024;

/// Maximum CPI instruction data size. 10 KiB was chosen to ensure that CPI
/// instructions are not more limited than transaction instructions if the size
/// of transactions is doubled in the future.
///
/// Copied from agave
pub const MAX_CPI_INSTRUCTION_DATA_LEN: usize = 10 * ONE_KB;

/// Maximum CPI instruction accounts. 255 was chosen to ensure that instruction
/// accounts are always within the maximum instruction account limit for SBF
/// program instructions.
///
/// Copied from agave
pub const MAX_CPI_INSTRUCTION_ACCOUNTS: u8 = u8::MAX;

/// Maximum number of account info structs that can be used in a single CPI
/// invocation. A limit on account info structs is effectively the same as
/// limiting the number of unique accounts. 128 was chosen to match the max
/// number of locked accounts per transaction (MAX_TX_ACCOUNT_LOCKS).
///
/// Copied from agave
pub const MAX_CPI_ACCOUNT_INFOS: usize = 128;

#[inline]
pub fn invoke_signed<const MAX_ACCOUNTS: usize, const MAX_CPI_ACCOUNTS: usize>(
    accounts: &mut Accounts<'_, MAX_ACCOUNTS>,
    Instr {
        prog,
        data,
        accounts: cpi_accounts,
    }: Instr<'_, '_>,
    signers_seeds: &[PdaSigner],
) -> Result<(), ProgramError> {
    invoke_signed_accounts_slice::<MAX_ACCOUNTS, MAX_CPI_ACCOUNTS>(
        accounts,
        prog,
        data,
        cpi_accounts,
        signers_seeds,
    )
}

#[inline]
pub fn invoke_signed_accounts_slice<const MAX_ACCOUNTS: usize, const MAX_CPI_ACCOUNTS: usize>(
    accounts: &mut Accounts<'_, MAX_ACCOUNTS>,
    cpi_prog: AccountHandle<'_>,
    cpi_ix_data: &[u8],
    cpi_accounts: &[(AccountHandle<'_>, AccountPerms)],
    signers_seeds: &[PdaSigner],
) -> Result<(), ProgramError> {
    invoke_signed_accounts_itr::<_, MAX_ACCOUNTS, MAX_CPI_ACCOUNTS>(
        accounts,
        cpi_prog,
        cpi_ix_data,
        cpi_accounts.iter().copied(),
        signers_seeds,
    )
}

#[inline]
pub fn invoke_signed_accounts_itr<
    'account,
    I,
    const MAX_ACCOUNTS: usize,
    const MAX_CPI_ACCOUNTS: usize,
>(
    accounts: &mut Accounts<'_, MAX_ACCOUNTS>,
    cpi_prog: AccountHandle<'_>,
    cpi_ix_data: &[u8],
    cpi_accounts: I,
    signers_seeds: &[PdaSigner],
) -> Result<(), ProgramError>
where
    I: IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>,
{
    const UNINIT_META: MaybeUninit<CpiAccountMeta> = MaybeUninit::uninit();
    const UNINIT_ACCOUNTS: MaybeUninit<CpiAccount> = MaybeUninit::uninit();

    let mut processed_metas = [UNINIT_META; MAX_CPI_ACCOUNTS];
    let mut processed_accounts = [UNINIT_ACCOUNTS; MAX_CPI_ACCOUNTS];
    let mut len = 0;

    for (handle, perm) in cpi_accounts {
        if len >= MAX_CPI_ACCOUNTS {
            return Err(ProgramError::InvalidArgument);
        }
        let acc = accounts.get(handle);
        processed_metas[len].write(CpiAccountMeta::new(acc, perm));
        // we technically dont need to pass duplicate AccountInfos
        // but making metas correspond 1:1 with accounts just makes it easier.
        //
        // We've also unfortunately erased duplicate flag info when
        // creating the `Accounts` struct.
        processed_accounts[len].write(CpiAccount::from_account_ref(acc));
        len += 1;
    }

    let prog_id = accounts.get(cpi_prog).key();

    invoke_signed_raw(
        prog_id,
        cpi_ix_data,
        unsafe { core::slice::from_raw_parts(processed_metas.as_ptr().cast(), len) },
        unsafe { core::slice::from_raw_parts(processed_accounts.as_ptr().cast(), len) },
        signers_seeds,
    )
}

#[inline]
pub fn invoke_signed_raw(
    prog_id: &[u8; 32],
    ix_data: &[u8],
    metas: &[CpiAccountMeta<'_>],
    accounts: &[CpiAccount<'_, '_>],
    signers_seeds: &[PdaSigner],
) -> Result<(), ProgramError> {
    #[cfg(target_os = "solana")]
    {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        struct CpiInstruction<'account> {
            /// Public key of the program.
            program_id: *const [u8; 32],

            /// Accounts expected by the program instruction.
            metas: *const CpiAccountMeta<'account>,

            /// Number of accounts expected by the program instruction.
            metas_len: u64,

            /// Data expected by the program instruction.
            data: *const u8,

            /// Length of the data expected by the program instruction.
            data_len: u64,
        }

        let ix = CpiInstruction {
            program_id: prog_id.as_ptr().cast(),
            metas: metas.as_ptr(),
            metas_len: metas.len() as u64,
            data: ix_data.as_ptr(),
            data_len: ix_data.len() as u64,
        };
        let res = unsafe {
            jiminy_syscall::sol_invoke_signed_c(
                core::ptr::from_ref(&ix).cast(),
                accounts.as_ptr().cast(),
                accounts.len() as u64,
                signers_seeds.as_ptr().cast(),
                signers_seeds.len() as u64,
            )
        };
        match res {
            0 => Ok(()),
            res => Err(res.into()),
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box((prog_id, metas, ix_data, accounts, signers_seeds));
        unreachable!()
    }
}
