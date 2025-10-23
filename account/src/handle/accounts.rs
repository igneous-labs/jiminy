use core::{mem::MaybeUninit, slice};

use crate::{Abr, AccountHandle, MAX_TX_ACCOUNTS};

// NB: MAX_ACCOUNTS should be able to fit into a u8, but its actually
// usually more CU efficient to use usize or u32 because ebpf only has
// 32-bit and 64-bit ALUs, so any ops with u8 might be preceded with/succeeded by
// ops like zeroing out the high bits of the register etc

/// This newtype must be consumed to create an [`Abr`], guaranteeing only 1 `Abr` per program
#[derive(Debug)]
#[repr(transparent)]
pub struct DeserAccounts<'account, const MAX_ACCOUNTS: usize>(
    pub(crate) Accounts<'account, MAX_ACCOUNTS>,
);

impl<'account, const MAX_ACCOUNTS: usize> DeserAccounts<'account, MAX_ACCOUNTS> {
    /// Entrypoint start
    #[inline(always)]
    pub const fn etp_start(self) -> (Abr, Accounts<'account, MAX_ACCOUNTS>) {
        (Abr::new(), self.0)
    }
}

/// An opaque sequence of accounts passed to the instruction by the runtime.
///
/// It dispenses [`AccountHandle`]s that then allow consumers to borrow [`Account`]s
/// either mutably or immutably in a safe way
///
/// `MAX_TX_ACCOUNTS` is max capacity of accounts, must be <= 255.
///
/// The only way to legally obtain this struct is using [`crate::deser_accounts`]
#[derive(Debug)]
pub struct Accounts<'account, const MAX_ACCOUNTS: usize = MAX_TX_ACCOUNTS> {
    pub(crate) accounts: [MaybeUninit<AccountHandle<'account>>; MAX_ACCOUNTS],
    pub(crate) len: usize,
}

impl<'account, const MAX_ACCOUNTS: usize> Accounts<'account, MAX_ACCOUNTS> {
    #[inline(always)]
    pub const fn as_slice(&self) -> &[AccountHandle<'account>] {
        unsafe { slice::from_raw_parts(self.accounts.as_ptr().cast(), self.len) }
    }

    // do not make as_mut_slice() because the array of account pointers
    // should not be mutable - each should always point to the same Account
}
