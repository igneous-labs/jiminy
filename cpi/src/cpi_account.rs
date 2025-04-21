use jiminy_account::Account;

/// An `Account` for CPI invocations.
///
/// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct CpiAccount {
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
}

impl CpiAccount {
    #[inline(always)]
    pub(crate) fn from_mut_account(acc: *mut Account) -> Self {
        unsafe {
            Self {
                key: Account::key_ptr(acc),
                lamports: Account::lamports_ptr(acc),
                data_len: Account::data_len_from_ptr(acc),
                data: Account::data_ptr(acc),
                owner: Account::owner_ptr(acc),
                rent_epoch: u64::MAX,
                is_signer: Account::is_signer_from_ptr(acc),
                is_writable: Account::is_writable_from_ptr(acc),
                is_executable: Account::is_executable_from_ptr(acc),
            }
        }
    }
}
