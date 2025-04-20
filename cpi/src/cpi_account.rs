use core::marker::PhantomData;

use jiminy_account::Account;

/// An `Account` for CPI invocations.
///
/// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpiAccount<'borrow> {
    /// Public key of the account.
    /// *const, shouldnt ever be modified
    key: *const [u8; 32],

    /// Number of lamports owned by this account.
    /// *mut because CPI may modify this.
    lamports: *mut u64,

    /// Length of data in bytes.
    data_len: u64,

    /// On-chain data within this account.
    /// *mut because CPI may modify this.
    data: *mut u8,

    /// Program that owns this account.
    /// *mut because CPI may modify this.
    owner: *mut [u8; 32],

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
    _account: PhantomData<&'borrow mut Account>,
}

impl CpiAccount<'_> {
    #[inline(always)]
    pub fn from_mut_account(account: &mut Account) -> Self {
        Self {
            key: account.key(),
            lamports: account.lamports_ref_mut(),
            data_len: account.data_len_u64(),
            data: account.data_mut().as_mut_ptr(),
            owner: account.owner_ref_mut(),
            rent_epoch: u64::MAX,
            is_signer: account.is_signer(),
            is_writable: account.is_writable(),
            is_executable: account.is_executable(),
            _account: PhantomData,
        }
    }
}

impl From<&mut Account> for CpiAccount<'_> {
    #[inline(always)]
    fn from(account: &mut Account) -> Self {
        Self::from_mut_account(account)
    }
}
