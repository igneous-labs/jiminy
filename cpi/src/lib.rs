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
    pub fn invoke_signed<'account, const MAX_ACCOUNTS: usize>(
        &mut self,
        accounts: &mut Accounts<'account, MAX_ACCOUNTS>,
        cpi_prog_id: &[u8; 32],
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>,
        signers_seeds: &[PdaSigner],
    ) -> Result<(), ProgramError> {
        let builder = CpiBuilder::new(self, accounts)
            .with_prog_id(cpi_prog_id)
            .with_ix_data(cpi_ix_data)
            .with_pda_signers(signers_seeds)
            .with_accounts(cpi_accounts)?;
        unsafe { builder.build() }.invoke(accounts)
    }

    /// In general this is more CU optimal than [`Self::invoke_signed`]
    /// if program is available as an account in the context
    /// because it can avoid a copy of its pubkey
    #[inline]
    pub fn invoke_signed_handle<'account, const MAX_ACCOUNTS: usize>(
        &mut self,
        accounts: &mut Accounts<'account, MAX_ACCOUNTS>,
        cpi_prog: AccountHandle<'account>,
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = (AccountHandle<'account>, AccountPerms)>,
        signers_seeds: &[PdaSigner],
    ) -> Result<(), ProgramError> {
        let builder = CpiBuilder::new(self, accounts)
            .with_prog_handle(cpi_prog)
            .with_ix_data(cpi_ix_data)
            .with_pda_signers(signers_seeds)
            .with_accounts(cpi_accounts)?;
        unsafe { builder.build() }.invoke(accounts)
    }

    /// CPI, but unlike [`Self::invoke_signed`], simply forwards the [`AccountPerms`] of each
    /// account within the `accounts` context instead of relying on another source.
    ///
    /// As such, this should not be used with any PDA signers, because a PDA signer should
    /// have is_signer=false in the invoking program's context, but will need to have
    /// is_signer set to true when CPI-ing. Use [`Self::invoke_signed`] instead.
    #[inline]
    pub fn invoke_fwd<'account, const MAX_ACCOUNTS: usize>(
        &mut self,
        accounts: &mut Accounts<'account, MAX_ACCOUNTS>,
        cpi_prog_id: &[u8; 32],
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = AccountHandle<'account>>,
    ) -> Result<(), ProgramError> {
        let builder = CpiBuilder::new(self, accounts)
            .with_prog_id(cpi_prog_id)
            .with_ix_data(cpi_ix_data)
            .with_accounts_fwd(cpi_accounts)?;
        unsafe { builder.build() }.invoke(accounts)
    }

    /// In general this is more CU optimal than [`Self::invoke_signed`]
    /// if program is available as an account in the context
    /// because it can avoid a copy of its pubkey
    #[inline]
    pub fn invoke_fwd_handle<'account, const MAX_ACCOUNTS: usize>(
        &mut self,
        accounts: &mut Accounts<'account, MAX_ACCOUNTS>,
        cpi_prog: AccountHandle<'account>,
        cpi_ix_data: &[u8],
        cpi_accounts: impl IntoIterator<Item = AccountHandle<'account>>,
    ) -> Result<(), ProgramError> {
        let builder = CpiBuilder::new(self, accounts)
            .with_prog_handle(cpi_prog)
            .with_ix_data(cpi_ix_data)
            .with_accounts_fwd(cpi_accounts)?;
        unsafe { builder.build() }.invoke(accounts)
    }
}

/// Lower level API for customizing derivation of CPI data from [`Accounts`]
#[derive(Debug)]
pub struct CpiBuilder<
    'cpi,
    'data,
    'accounts,
    const MAX_CPI_ACCOUNTS: usize,
    const MAX_ACCOUNTS: usize,
    const HAS_PROG_ID: bool,
> {
    accounts: &'data Accounts<'accounts, MAX_ACCOUNTS>,
    cpi: PreparedCpi<'cpi, MAX_CPI_ACCOUNTS>,
}

/// Constructors
impl<'cpi, 'data, 'accounts, const MAX_CPI_ACCOUNTS: usize, const MAX_ACCOUNTS: usize>
    CpiBuilder<'cpi, 'data, 'accounts, MAX_CPI_ACCOUNTS, MAX_ACCOUNTS, false>
{
    #[inline]
    pub const fn new(
        cpi: &'cpi mut Cpi<MAX_CPI_ACCOUNTS>,
        accounts: &'data Accounts<'accounts, MAX_ACCOUNTS>,
    ) -> Self {
        Self {
            accounts,
            cpi: PreparedCpi {
                cpi,
                accs_len: 0,
                prog_id: core::ptr::null(),
                data: [].as_ptr(),
                data_len: 0,
                signers_seeds: [].as_ptr(),
                signers_seeds_len: 0,
            },
        }
    }
}

impl<
        'cpi,
        'data,
        'accounts,
        const MAX_CPI_ACCOUNTS: usize,
        const MAX_ACCOUNTS: usize,
        const HAS_PROG_ID: bool,
    > CpiBuilder<'cpi, 'data, 'accounts, MAX_CPI_ACCOUNTS, MAX_ACCOUNTS, HAS_PROG_ID>
{
    // prog ID

    #[inline]
    pub fn try_with_prog_id<E>(
        self,
        derive_prog_id: impl FnOnce(
            &'data Accounts<'accounts, MAX_ACCOUNTS>,
        ) -> Result<&'data [u8; 32], E>,
    ) -> Result<CpiBuilder<'cpi, 'data, 'accounts, MAX_CPI_ACCOUNTS, MAX_ACCOUNTS, true>, E> {
        let Self {
            mut cpi, accounts, ..
        } = self;
        let prog_id = derive_prog_id(accounts)?;
        cpi.prog_id = prog_id;
        Ok(CpiBuilder { cpi, accounts })
    }

    #[inline]
    pub fn with_prog_handle(
        self,
        handle: AccountHandle<'accounts>,
    ) -> CpiBuilder<'cpi, 'data, 'accounts, MAX_CPI_ACCOUNTS, MAX_ACCOUNTS, true> {
        self.try_with_prog_id(|a| Ok::<_, Infallible>(a.get(handle).key()))
            .unwrap()
    }

    #[inline]
    pub fn with_prog_id(
        self,
        prog_id: &'data [u8; 32],
    ) -> CpiBuilder<'cpi, 'data, 'accounts, MAX_CPI_ACCOUNTS, MAX_ACCOUNTS, true> {
        self.try_with_prog_id(|_a| Ok::<_, Infallible>(prog_id))
            .unwrap()
    }

    // ix data

    #[inline]
    pub fn try_with_ix_data<E>(
        self,
        derive_ix_data: impl FnOnce(&'data Accounts<'accounts, MAX_ACCOUNTS>) -> Result<&'data [u8], E>,
    ) -> Result<Self, E> {
        let Self { accounts, mut cpi } = self;
        let data = derive_ix_data(accounts)?;
        cpi.data = data.as_ptr();
        cpi.data_len = data.len() as u64;
        Ok(Self { cpi, accounts })
    }

    #[inline]
    pub fn with_ix_data(self, ix_data: &'data [u8]) -> Self {
        self.try_with_ix_data(|_a| Ok::<_, Infallible>(ix_data))
            .unwrap()
    }

    // signers

    #[inline]
    pub fn try_with_pda_signers<E>(
        self,
        derive_pda_signers: impl FnOnce(
            &'data Accounts<'accounts, MAX_ACCOUNTS>,
        ) -> Result<&'data [PdaSigner<'data, 'data>], E>,
    ) -> Result<Self, E> {
        let Self { accounts, mut cpi } = self;
        let signers = derive_pda_signers(accounts)?;
        cpi.signers_seeds = signers.as_ptr().cast();
        cpi.signers_seeds_len = signers.len() as u64;
        Ok(Self { cpi, accounts })
    }

    #[inline]
    pub fn with_pda_signers(self, signers: &'data [PdaSigner]) -> Self {
        self.try_with_pda_signers(|_a| Ok::<_, Infallible>(signers))
            .unwrap()
    }

    // accounts

    #[inline]
    pub fn try_with_accounts<I: IntoIterator<Item = (AccountHandle<'accounts>, AccountPerms)>>(
        self,
        derive_accounts: impl FnOnce(
            &'data Accounts<'accounts, MAX_ACCOUNTS>,
        ) -> Result<I, ProgramError>,
    ) -> Result<Self, ProgramError> {
        let Self { accounts, mut cpi } = self;
        let len = derive_accounts(accounts)?
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
                cpi.cpi.metas[len].write(CpiAccountMeta::new(acc, perm));
                // we technically dont need to pass duplicate AccountInfos
                // but making metas correspond 1:1 with accounts just makes it easier.
                // This allows us to store one less u64 in `PreparedCpi`
                // thanks to invariant of metas.len() == accounts.len()
                //
                // We've also unfortunately erased duplicate flag info when
                // creating the `Accounts` struct.
                cpi.cpi.accounts[len].write(CpiAccount::from_ptr(acc));
                Ok(len + 1)
            })?;
        cpi.accs_len = len as u64;
        Ok(Self { cpi, accounts })
    }

    #[inline]
    pub fn with_accounts(
        self,
        accounts: impl IntoIterator<Item = (AccountHandle<'accounts>, AccountPerms)>,
    ) -> Result<Self, ProgramError> {
        self.try_with_accounts(|_a| Ok(accounts))
    }

    #[inline]
    pub fn try_with_accounts_fwd<I: IntoIterator<Item = AccountHandle<'accounts>>>(
        self,
        derive_accounts: impl FnOnce(
            &'data Accounts<'accounts, MAX_ACCOUNTS>,
        ) -> Result<I, ProgramError>,
    ) -> Result<Self, ProgramError> {
        let Self { accounts, mut cpi } = self;
        let len = derive_accounts(accounts)?
            .into_iter()
            .try_fold(0, |len, handle| {
                if len >= MAX_CPI_ACCOUNTS {
                    return Err(ProgramError::from_builtin(
                        BuiltInProgramError::InvalidArgument,
                    ));
                }
                let acc = accounts.get_ptr(handle);

                // this fn's code should be the exact same as
                // [`Self::try_with_accounts`] except for this line here:
                cpi.cpi.metas[len].write(CpiAccountMeta::fwd(acc));

                cpi.cpi.accounts[len].write(CpiAccount::from_ptr(acc));
                Ok(len + 1)
            })?;
        cpi.accs_len = len as u64;
        Ok(Self { cpi, accounts })
    }

    #[inline]
    pub fn with_accounts_fwd(
        self,
        accounts: impl IntoIterator<Item = AccountHandle<'accounts>>,
    ) -> Result<Self, ProgramError> {
        self.try_with_accounts_fwd(|_a| Ok(accounts))
    }
}

impl<'cpi, const MAX_CPI_ACCOUNTS: usize, const MAX_ACCOUNTS: usize>
    CpiBuilder<'cpi, '_, '_, MAX_CPI_ACCOUNTS, MAX_ACCOUNTS, true>
{
    /// # Safety
    /// Until the returned [`PreparedCpi`] is dropped, `accounts` must not be mutated after
    /// calling this method apart from the returned struct's [`PreparedCpi::invoke`], else UB.
    #[inline]
    pub const unsafe fn build(self) -> PreparedCpi<'cpi, MAX_CPI_ACCOUNTS> {
        self.cpi
    }
}

/// A CPI that's ready to invoke.
///
/// Only way to obtain this struct is via [`CpiBuilder::build`]
#[derive(Debug)]
pub struct PreparedCpi<'cpi, const MAX_CPI_ACCOUNTS: usize> {
    cpi: &'cpi mut Cpi<MAX_CPI_ACCOUNTS>,
    accs_len: u64,
    prog_id: *const [u8; 32],
    data: *const u8,
    data_len: u64,
    signers_seeds: *const u8,
    signers_seeds_len: u64,
}

impl<const MAX_CPI_ACCOUNTS: usize> PreparedCpi<'_, MAX_CPI_ACCOUNTS> {
    /// Each `PreparedCpi` is only valid for one-time use because
    /// a CPI may realloc an account, invalidating its `CpiAccount::data_len`
    #[inline]
    pub fn invoke<const MAX_ACCOUNTS: usize>(
        self,
        _accounts: &mut Accounts<'_, MAX_ACCOUNTS>,
    ) -> Result<(), ProgramError> {
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
                program_id: self.prog_id,
                metas: self.cpi.metas.as_ptr().cast(),
                metas_len: self.accs_len,
                data: self.data,
                data_len: self.data_len,
            };

            // safety: mut borrow of `&mut Accounts` ensures
            // that no account is being borrowed elsewhere
            let res = unsafe {
                jiminy_syscall::sol_invoke_signed_c(
                    core::ptr::addr_of!(ix).cast(),
                    self.cpi.accounts.as_ptr().cast(),
                    self.accs_len,
                    self.signers_seeds,
                    self.signers_seeds_len,
                )
            };
            match core::num::NonZeroU64::new(res) {
                None => Ok(()),
                Some(err) => Err(err.into()),
            }
        }

        #[cfg(not(target_os = "solana"))]
        {
            unreachable!()
        }
    }
}
