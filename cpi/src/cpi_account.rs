use jiminy_account::UnsafeAccount;

/// An `Account` for CPI invocations.
///
/// This struct has the memory layout as expected by `sol_invoke_signed_c` syscall.
///
/// Note that the struct defn is vastly different from [`Account`]
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
    pub(crate) fn from_unsafe(acc: UnsafeAccount<'_>) -> Self {
        unsafe {
            Self {
                key: acc.key_ptr(),
                lamports: acc.lamports_ptr(),
                data_len: acc.data_len(),
                data: acc.data_ptr(),
                owner: acc.owner_ptr(),
                rent_epoch: u64::MAX,
                is_signer: acc.is_signer(),
                is_writable: acc.is_writable(),
                is_executable: acc.is_executable(),
            }
        }
    }
}
