use core::marker::PhantomData;

use jiminy_account::{Account, Accounts};

/// Account permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountPerms {
    pub is_writable: bool,
    pub is_signer: bool,
}

/// An `AccountMeta` for CPI invocations.
///
/// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct CpiAccountMeta<'borrow> {
    /// `*const`, shouldnt ever be modified.
    pubkey: *const [u8; 32],
    is_writable: bool,
    is_signer: bool,

    /// This is tied to the borrow of the entire [`Accounts`] collection,
    /// not the individual [`Account`]
    _accounts: PhantomData<&'borrow Account>,
}

impl<'borrow> CpiAccountMeta<'borrow> {
    #[inline(always)]
    pub(crate) fn new<const N: usize>(
        _accounts: &'borrow Accounts<N>,
        acc: *mut Account,
        AccountPerms {
            is_writable,
            is_signer,
        }: AccountPerms,
    ) -> Self {
        Self {
            pubkey: unsafe { Account::key_ptr(acc) },
            is_writable,
            is_signer,
            _accounts: PhantomData,
        }
    }
}
