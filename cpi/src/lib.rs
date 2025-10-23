#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

//! All the invocation functions take `&mut Accounts` as param
//! because we must have exclusive access to accounts since CPI may mutate accounts

use core::{convert::Infallible, mem::MaybeUninit};

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

pub use cpi_account_meta::*;

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
    pub fn invoke_signed<'account>(
        &mut self,
        abr: &mut Abr,
        cpi_prog_id: &[u8; 32],
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>,
        signers_seeds: &[PdaSigner],
    ) -> Result<(), ProgramError> {
        CpiBuilder::new(self, abr)
            .with_prog_id(cpi_prog_id)
            .with_ix_data(cpi_ix_data)
            .with_pda_signers(signers_seeds)
            .with_accounts(cpi_accounts)?
            .invoke()
    }

    /// In general this is more CU optimal than [`Self::invoke_signed`]
    /// if program is available as an account in the context
    /// because it can avoid a copy of its pubkey
    #[inline]
    pub fn invoke_signed_handle<'account>(
        &mut self,
        abr: &mut Abr,
        cpi_prog: AccountHandle<'account>,
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>,
        signers_seeds: &[PdaSigner],
    ) -> Result<(), ProgramError> {
        CpiBuilder::new(self, abr)
            .with_prog_handle(cpi_prog)
            .with_ix_data(cpi_ix_data)
            .with_pda_signers(signers_seeds)
            .with_accounts(cpi_accounts)?
            .invoke()
    }

    /// CPI, but unlike [`Self::invoke_signed`], simply forwards the [`AccountPerms`] of each
    /// account within the `accounts` context instead of relying on another source.
    ///
    /// As such, this should not be used with any PDA signers, because a PDA signer should
    /// have is_signer=false in the invoking program's context, but will need to have
    /// is_signer set to true when CPI-ing. Use [`Self::invoke_signed`] instead.
    #[inline]
    pub fn invoke_fwd<'account>(
        &mut self,
        abr: &mut Abr,
        cpi_prog_id: &[u8; 32],
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = AccountHandle<'account>>,
    ) -> Result<(), ProgramError> {
        CpiBuilder::new(self, abr)
            .with_prog_id(cpi_prog_id)
            .with_ix_data(cpi_ix_data)
            .with_accounts_fwd(cpi_accounts)?
            .invoke()
    }

    /// In general this is more CU optimal than [`Self::invoke_signed`]
    /// if program is available as an account in the context
    /// because it can avoid a copy of its pubkey
    #[inline]
    pub fn invoke_fwd_handle<'account>(
        &mut self,
        abr: &mut Abr,
        cpi_prog: AccountHandle<'account>,
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = AccountHandle<'account>>,
    ) -> Result<(), ProgramError> {
        CpiBuilder::new(self, abr)
            .with_prog_handle(cpi_prog)
            .with_ix_data(cpi_ix_data)
            .with_accounts_fwd(cpi_accounts)?
            .invoke()
    }
}

/// Lower level API for customizing derivation of CPI data from [`Accounts`]
#[derive(Debug)]
pub struct CpiBuilder<'cpi, const MAX_CPI_ACCOUNTS: usize, const HAS_PROG_ID: bool> {
    abr: &'cpi mut Abr,
    cpi: &'cpi mut Cpi<MAX_CPI_ACCOUNTS>,
    accs_len: u64,
    prog_id: *const [u8; 32],
    data: *const u8,
    data_len: u64,
    signers_seeds: *const u8,
    signers_seeds_len: u64,
}

/// Constructors
impl<'cpi, const MAX_CPI_ACCOUNTS: usize> CpiBuilder<'cpi, MAX_CPI_ACCOUNTS, false> {
    #[inline]
    pub const fn new(cpi: &'cpi mut Cpi<MAX_CPI_ACCOUNTS>, abr: &'cpi mut Abr) -> Self {
        Self {
            abr,
            cpi,
            accs_len: 0,
            prog_id: core::ptr::null(),
            data: core::ptr::null(),
            data_len: 0,
            signers_seeds: core::ptr::null(),
            signers_seeds_len: 0,
        }
    }
}

impl<'cpi, const MAX_CPI_ACCOUNTS: usize, const HAS_PROG_ID: bool>
    CpiBuilder<'cpi, MAX_CPI_ACCOUNTS, HAS_PROG_ID>
{
    // prog ID

    #[inline]
    pub fn try_with_derive_prog_id<E>(
        self,
        derive_prog_id: impl for<'a> FnOnce(&'a Abr) -> Result<&'a [u8; 32], E>,
    ) -> Result<CpiBuilder<'cpi, MAX_CPI_ACCOUNTS, true>, E> {
        let Self {
            abr,
            cpi,
            accs_len,
            data,
            data_len,
            signers_seeds,
            signers_seeds_len,
            prog_id: _,
        } = self;
        let prog_id = derive_prog_id(abr)?.as_ptr().cast();
        Ok(CpiBuilder {
            cpi,
            abr,
            accs_len,
            prog_id,
            data,
            data_len,
            signers_seeds,
            signers_seeds_len,
        })
    }

    #[inline]
    pub fn with_derive_prog_id(
        self,
        derive_prog_id: impl for<'a> FnOnce(&'a Abr) -> &'a [u8; 32],
    ) -> CpiBuilder<'cpi, MAX_CPI_ACCOUNTS, true> {
        self.try_with_derive_prog_id(|a| Ok::<_, Infallible>(derive_prog_id(a)))
            .unwrap()
    }

    #[inline]
    pub fn with_prog_handle(
        self,
        handle: AccountHandle<'_>,
    ) -> CpiBuilder<'cpi, MAX_CPI_ACCOUNTS, true> {
        self.with_derive_prog_id(|a| a.get(handle).key())
    }

    #[inline]
    pub fn with_prog_id<'a: 'cpi>(
        self,
        prog_id: &'a [u8; 32],
    ) -> CpiBuilder<'cpi, MAX_CPI_ACCOUNTS, true> {
        let Self {
            abr,
            cpi,
            accs_len,
            data,
            data_len,
            signers_seeds,
            signers_seeds_len,
            prog_id: _,
        } = self;
        CpiBuilder {
            cpi,
            abr,
            accs_len,
            prog_id,
            data,
            data_len,
            signers_seeds,
            signers_seeds_len,
        }
    }

    // ix data

    #[inline]
    pub fn try_with_derive_ix_data<E>(
        mut self,
        derive_ix_data: impl for<'a> FnOnce(&'a Abr) -> Result<&'a [u8], E>,
    ) -> Result<Self, E> {
        let data = derive_ix_data(self.abr)?;
        self.data = data.as_ptr();
        self.data_len = data.len() as u64;
        Ok(self)
    }

    #[inline]
    pub fn with_derive_ix_data(
        self,
        derive_ix_data: impl for<'a> FnOnce(&'a Abr) -> &'a [u8],
    ) -> Self {
        self.try_with_derive_ix_data(|a| Ok::<_, Infallible>(derive_ix_data(a)))
            .unwrap()
    }

    #[inline]
    pub fn with_ix_data<'a: 'cpi>(mut self, ix_data: &'a [u8]) -> Self {
        self.data = ix_data.as_ptr();
        self.data_len = ix_data.len() as u64;
        self
    }

    // signers

    // Not much practicality for this right now due to the 2 levels of pointer indirection
    // for &[PdaSigner] so its really hard to something like point to seeds stored in account data
    #[inline]
    pub fn try_with_derive_pda_signers<E>(
        mut self,
        derive_pda_signers: impl for<'a> FnOnce(&'a Abr) -> Result<&'a [PdaSigner<'a, 'a>], E>,
    ) -> Result<Self, E> {
        let signers = derive_pda_signers(self.abr)?;
        self.signers_seeds = signers.as_ptr().cast();
        self.signers_seeds_len = signers.len() as u64;
        Ok(self)
    }

    #[inline]
    pub fn with_derive_pda_signers(
        self,
        derive_pda_signers: impl for<'a> FnOnce(&'a Abr) -> &'a [PdaSigner<'a, 'a>],
    ) -> Self {
        self.try_with_derive_pda_signers(|a| Ok::<_, Infallible>(derive_pda_signers(a)))
            .unwrap()
    }

    #[inline]
    pub fn with_pda_signers<'a: 'cpi>(mut self, signers: &'a [PdaSigner]) -> Self {
        self.signers_seeds = signers.as_ptr().cast();
        self.signers_seeds_len = signers.len() as u64;
        self
    }

    // accounts

    #[inline]
    pub fn with_accounts<
        'accounts,
        I: IntoIterator<Item = (AccountHandle<'accounts>, AccountPerms)>,
    >(
        mut self,
        accounts: I,
    ) -> Result<Self, ProgramError> {
        let len = accounts.into_iter().try_fold(0, |len, (handle, perm)| {
            if len >= MAX_CPI_ACCOUNTS {
                return Err(ProgramError::from_builtin(
                    BuiltInProgramError::InvalidArgument,
                ));
            }
            let acc = self.abr.get_ptr(handle);
            // index-safety: bounds checked against MAX_CPI_ACCOUNTS above
            // write-safety: CpiAccountMeta and CpiAccount are Copy,
            // dont care about overwriting old data
            self.cpi.metas[len].write(CpiAccountMeta::new(acc, perm));
            // we technically dont need to pass duplicate AccountInfos
            // but making metas correspond 1:1 with accounts just makes it easier.
            // This allows us to store one less u64
            // thanks to invariant of metas.len() == accounts.len()
            //
            // We've also unfortunately erased duplicate flag info when
            // creating the `Accounts` struct.
            self.cpi.accounts[len].write(CpiAccount::from_ptr(acc));
            Ok(len + 1)
        })?;
        self.accs_len = len as u64;
        Ok(self)
    }

    #[inline]
    pub fn with_accounts_fwd<'accounts, I: IntoIterator<Item = AccountHandle<'accounts>>>(
        mut self,
        accounts: I,
    ) -> Result<Self, ProgramError> {
        let len = accounts.into_iter().try_fold(0, |len, handle| {
            if len >= MAX_CPI_ACCOUNTS {
                return Err(ProgramError::from_builtin(
                    BuiltInProgramError::InvalidArgument,
                ));
            }
            let acc = self.abr.get_ptr(handle);

            // this fn's code should be the exact same as
            // [`Self::try_with_accounts`] except for this line here:
            self.cpi.metas[len].write(CpiAccountMeta::fwd(acc));

            self.cpi.accounts[len].write(CpiAccount::from_ptr(acc));
            Ok(len + 1)
        })?;
        self.accs_len = len as u64;
        Ok(self)
    }
}

impl<const MAX_CPI_ACCOUNTS: usize> CpiBuilder<'_, MAX_CPI_ACCOUNTS, true> {
    #[inline]
    pub fn invoke(self) -> Result<(), ProgramError> {
        let Self {
            cpi,
            accs_len,
            prog_id,
            data,
            data_len,
            signers_seeds,
            signers_seeds_len,
            // not used, just here to guarantee exclusive borrow of Accounts
            abr: _,
        } = self;
        #[cfg(target_os = "solana")]
        {
            /// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
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
                program_id: prog_id,
                metas: cpi.metas.as_ptr().cast(),
                metas_len: accs_len,
                data,
                data_len,
            };

            // safety: mut borrow of `&mut Accounts` ensures
            // that no account is being borrowed elsewhere
            let res = unsafe {
                jiminy_syscall::sol_invoke_signed_c(
                    core::ptr::addr_of!(ix).cast(),
                    cpi.accounts.as_ptr().cast(),
                    accs_len,
                    signers_seeds,
                    signers_seeds_len,
                )
            };
            match core::num::NonZeroU64::new(res) {
                None => Ok(()),
                Some(err) => Err(err.into()),
            }
        }

        #[cfg(not(target_os = "solana"))]
        {
            // avoid unused warnings
            core::hint::black_box((
                cpi,
                accs_len,
                prog_id,
                data,
                data_len,
                signers_seeds,
                signers_seeds_len,
            ));
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused)]
    fn lifetime_comptime_playground<'cpi, 'accounts>(
        abr: &'cpi mut Abr,
        accounts: &[AccountHandle<'accounts>],
        cpi: &'cpi mut Cpi,
        slice: &[u8],
    ) -> CpiBuilder<'cpi, 48, false> {
        let h = *accounts.first().unwrap();
        let mut signer: PdaSeedArr<'_> = PdaSeedArr::new();

        CpiBuilder::new(cpi, abr)
            .with_derive_ix_data(|a| a.get(h).data())
            .with_accounts_fwd(accounts.iter().copied())
            .unwrap()
    }
}
