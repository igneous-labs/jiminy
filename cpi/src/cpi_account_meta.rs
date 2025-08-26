use jiminy_account::Account;

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
pub(crate) struct CpiAccountMeta {
    /// `*const`, shouldnt ever be modified.
    pubkey: *const [u8; 32],
    is_writable: bool,
    is_signer: bool,
}

impl CpiAccountMeta {
    #[inline(always)]
    pub(crate) fn new(
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
        }
    }

    /// Use the permissions of `acc` instead of having it from
    /// an arg like [`Self::new`]
    #[inline(always)]
    pub(crate) fn fwd(acc: *mut Account) -> Self {
        unsafe {
            Self {
                pubkey: Account::key_ptr(acc),
                is_writable: (*acc).is_writable(),
                is_signer: (*acc).is_signer(),
            }
        }
    }
}
