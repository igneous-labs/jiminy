use core::marker::PhantomData;

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
pub struct CpiAccountMeta<'borrow> {
    pubkey: *const [u8; 32],
    is_writable: bool,
    is_signer: bool,

    /// This struct is only valid while the [`Account`] it points to
    /// is borrowed. Assumes the [`Account`] pubkey will not be mutated
    /// (runtime disallows this)
    _account: PhantomData<&'borrow Account<'borrow>>,
}

impl<'borrow> CpiAccountMeta<'borrow> {
    #[inline(always)]
    pub fn new(
        acc: &'borrow Account<'_>,
        AccountPerms {
            is_writable,
            is_signer,
        }: AccountPerms,
    ) -> Self {
        Self {
            pubkey: acc.key(),
            is_writable,
            is_signer,
            _account: PhantomData,
        }
    }
}
