#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

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

pub use cpi_account_meta::*;
pub use instruction::*;

use cpi_account::*;

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

/// Max number of CPI accounts for a [`Cpi`]
/// to fit on the stack.
///
/// To invoke CPIs with more accounts, increase the `MAX_CPI_ACCOUNTS`
/// const generic and create a [`Box<Cpi>`] on the heap
pub const MAX_CPI_ACCOUNTS_STACK_ONLY: usize = 48;

/// A CPI invocation, contains the [`CpiAccountMeta`] and [`CpiAccount`]
/// buffers required to pass to the syscall.
///
/// Must be instantiated to make a CPI. this allows it to be placed on the heap for
/// large values of `MAX_CPI_ACCOUNTS`. This also allows the underlying buffers to be
/// reused for multiple CPIs.
#[derive(Debug, Clone)]
pub struct Cpi<const MAX_CPI_ACCOUNTS: usize = MAX_CPI_ACCOUNTS_STACK_ONLY> {
    metas: [MaybeUninit<CpiAccountMeta>; MAX_CPI_ACCOUNTS],
    accounts: [MaybeUninit<CpiAccount>; MAX_CPI_ACCOUNTS],
}

impl<const MAX_CPI_ACCOUNTS: usize> Cpi<MAX_CPI_ACCOUNTS> {
    #[inline(always)]
    pub const fn new() -> Self {
        const UNINIT_META: MaybeUninit<CpiAccountMeta> = MaybeUninit::uninit();
        const UNINIT_ACCOUNT: MaybeUninit<CpiAccount> = MaybeUninit::uninit();

        Self {
            metas: [UNINIT_META; MAX_CPI_ACCOUNTS],
            accounts: [UNINIT_ACCOUNT; MAX_CPI_ACCOUNTS],
        }
    }
}

impl<const MAX_CPI_ACCOUNTS: usize> Default for Cpi<MAX_CPI_ACCOUNTS> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_CPI_ACCOUNTS: usize> Cpi<MAX_CPI_ACCOUNTS> {
    // DO NOT #[inline(always)] invoke_signed.
    // #[inline] results in lower CUs and binary sizes

    #[inline]
    pub fn invoke_signed<'account, const MAX_ACCOUNTS: usize>(
        &mut self,
        accounts: &mut Accounts<'account, MAX_ACCOUNTS>,
        Instr {
            prog: cpi_prog,
            data: cpi_ix_data,
            accounts: cpi_accounts,
        }: Instr<'_, '_, impl IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>>,
        signers_seeds: &[PdaSigner],
    ) -> Result<(), ProgramError> {
        let cpi_prog_id = *accounts.get(cpi_prog).key();
        self.invoke_signed_raw(
            accounts,
            &cpi_prog_id,
            cpi_ix_data,
            cpi_accounts,
            signers_seeds,
        )
    }

    /// Same as [`Self::invoke_signed`], but with args exploded instead of
    /// in an [`Instr`] struct + doesn't require [`AccountHandle`] for program being invoked
    /// (useful for self CPIs)
    #[inline]
    pub fn invoke_signed_raw<'account, const MAX_ACCOUNTS: usize>(
        &mut self,
        accounts: &mut Accounts<'account, MAX_ACCOUNTS>,
        cpi_prog_id: &[u8; 32],
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>,
        signers_seeds: &[PdaSigner],
    ) -> Result<(), ProgramError> {
        let len = cpi_accounts
            .into_iter()
            .try_fold(0, |len, (handle, perm)| {
                if len >= MAX_CPI_ACCOUNTS {
                    return Err(ProgramError::from_builtin(
                        BuiltInProgramError::InvalidArgument,
                    ));
                }
                let acc = accounts.get_ptr(handle);
                // index-safety: bounds checked against MAX_CPI_ACCOUNTS above
                // write-safety: CpiAccountMeta and CpiAccount are Copy,
                // dont care about overwriting old data
                self.metas[len].write(CpiAccountMeta::new(acc, perm));
                // we technically dont need to pass duplicate AccountInfos
                // but making metas correspond 1:1 with accounts just makes it easier.
                //
                // We've also unfortunately erased duplicate flag info when
                // creating the `Accounts` struct.
                self.accounts[len].write(CpiAccount::from_mut_account(acc));
                Ok(len + 1)
            })?;

        unsafe {
            invoke_signed_raw(
                cpi_prog_id,
                cpi_ix_data,
                core::slice::from_raw_parts(self.metas.as_ptr().cast(), len),
                core::slice::from_raw_parts(self.accounts.as_ptr().cast(), len),
                signers_seeds,
            )
        }
    }
}

/// # Safety
/// - metas and accounts must be pointing to Accounts that are not currently borrowed
///   elsewhere, else UB. This is guaranteed by `&mut Accounts` in [`Cpi::invoke_signed`]
#[inline]
unsafe fn invoke_signed_raw(
    prog_id: &[u8; 32],
    ix_data: &[u8],
    metas: &[CpiAccountMeta],
    accounts: &[CpiAccount],
    signers_seeds: &[PdaSigner],
) -> Result<(), ProgramError> {
    #[cfg(target_os = "solana")]
    {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        struct CpiInstruction {
            /// Public key of the program.
            program_id: *const [u8; 32],

            /// Accounts expected by the program instruction.
            metas: *const CpiAccountMeta,

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
                core::ptr::addr_of!(ix).cast(),
                accounts.as_ptr().cast(),
                accounts.len() as u64,
                signers_seeds.as_ptr().cast(),
                signers_seeds.len() as u64,
            )
        };
        match core::num::NonZeroU64::new(res) {
            None => Ok(()),
            Some(err) => Err(err.into()),
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box((prog_id, metas, ix_data, accounts, signers_seeds));
        unreachable!()
    }
}
