use core::marker::PhantomData;

use jiminy_account::Account;

/// An `Account` for CPI invocations.
///
/// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpiAccount<'borrow> {
    // Public key of the account.
    key: *const [u8; 32],

    // Number of lamports owned by this account.
    lamports: *const u64,

    // Length of data in bytes.
    data_len: u64,

    // On-chain data within this account.
    data: *const u8,

    // Program that owns this account.
    owner: *const [u8; 32],

    // The epoch at which this account will next owe rent.
    rent_epoch: u64,

    // Transaction was signed by this account's key?
    is_signer: bool,

    // Is the account writable?
    is_writable: bool,

    // This account's data contains a loaded program (and is now read-only).
    is_executable: bool,

    /// This struct is only valid while the [`Account`] it points to
    /// is borrowed and valid.
    _account: PhantomData<&'borrow Account>,
}

impl<'borrow> CpiAccount<'borrow> {
    #[inline(always)]
    pub fn from_account_ref(account: &'borrow Account) -> Self {
        Self {
            key: account.key(),
            lamports: account.lamports_ref(),
            data_len: account.data_len_u64(),
            data: account.data().as_ptr(),
            owner: account.owner(),
            rent_epoch: u64::MAX,
            is_signer: account.is_signer(),
            is_writable: account.is_writable(),
            is_executable: account.is_executable(),
            _account: PhantomData,
        }
    }
}

impl<'borrow> From<&'borrow Account> for CpiAccount<'borrow> {
    #[inline(always)]
    fn from(account: &'borrow Account) -> Self {
        Self::from_account_ref(account)
    }
}
