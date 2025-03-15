use core::marker::PhantomData;

use jiminy_account::Account;

/// Account permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountPerms {
    WritableSigner,
    ReadonlySigner,
    Writable,
    Readonly,
}

impl AccountPerms {
    #[inline]
    pub const fn is_writable(&self) -> bool {
        match self {
            Self::Readonly | Self::ReadonlySigner => false,
            Self::Writable | Self::WritableSigner => true,
        }
    }

    #[inline]
    pub const fn is_signer(&self) -> bool {
        match self {
            Self::Readonly | Self::Writable => false,
            Self::ReadonlySigner | Self::WritableSigner => true,
        }
    }
}

/// An `AccountMeta` for CPI invocations.
///
/// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpiAccountMeta<'account> {
    pubkey: *const [u8; 32],
    is_writable: bool,
    is_signer: bool,

    /// This struct is only valid while the [`Account`] it points to
    /// is valid. Assumes the [`Account`] pubkey will not be mutated
    /// (runtime disallows this)
    _account: PhantomData<Account<'account>>,
}

impl<'account> CpiAccountMeta<'account> {
    #[inline]
    pub fn new(acc: &Account<'account>, role: AccountPerms) -> Self {
        Self {
            pubkey: acc.key(),
            is_writable: role.is_writable(),
            is_signer: role.is_signer(),
            _account: PhantomData,
        }
    }
}
