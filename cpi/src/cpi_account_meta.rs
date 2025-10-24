use jiminy_account::UnsafeAccount;

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
    pub(crate) const fn new(
        acc: UnsafeAccount<'_>,
        AccountPerms {
            is_writable,
            is_signer,
        }: AccountPerms,
    ) -> Self {
        Self {
            pubkey: unsafe { acc.key_ptr() },
            is_writable,
            is_signer,
        }
    }

    /// Use the permissions of `acc` instead of having it from
    /// an arg like [`Self::new`]
    #[inline(always)]
    pub(crate) const fn fwd(acc: UnsafeAccount<'_>) -> Self {
        unsafe {
            Self {
                pubkey: acc.key_ptr(),
                is_writable: acc.is_writable(),
                is_signer: acc.is_signer(),
            }
        }
    }
}
