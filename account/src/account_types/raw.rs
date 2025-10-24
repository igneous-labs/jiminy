/// A non-duplicate account as it is laid out in runtime serialized memory
///
/// Used primarily to calculate offsets.
#[derive(Debug)]
#[repr(C)]
pub struct RawAccount {
    pub(crate) _duplicate_flag: u8,

    /// Indicates whether the transaction was signed by this account.
    pub(crate) is_signer: u8,

    /// Indicates whether the account is writable.
    pub(crate) is_writable: u8,

    /// Indicates whether this account represents a program.
    pub(crate) is_executable: u8,

    /// The number of bytes this account has already grown by
    /// from its original size. A negative value means the account
    /// has shrunk
    ///
    /// Capped at [`crate::MAX_PERMITTED_DATA_INCREASE`].
    ///
    /// Overflow safety: solana accounts have a max data size of 10Mib,
    /// well within i32 range in either +/- direction.
    ///
    /// These 4 bytes here used to be struct padding bytes,
    /// until anza decided to repurpose them
    /// as scratch space for recording data to support realloc in 1.10.
    /// Guaranteed to be zero at entrypoint time.
    pub(crate) realloc_budget_used: i32,

    /// Public key of the account.
    pub(crate) key: [u8; 32],

    /// Program that owns this account. Modifiable by programs.
    pub(crate) owner: [u8; 32],

    /// The lamports in the account. Modifiable by programs.
    pub(crate) lamports: u64,

    /// Length of the data. Modifiable by programs.
    pub(crate) data_len: u64,
}

const _CHECK_ACCOUNT_RAW_SIZE: () = assert!(size_of::<RawAccount>() == 88);
const _CHECK_ACCOUN_RAW_ALIGN: () = assert!(align_of::<RawAccount>() == 8);

/// Returns the offset that should be decremented
/// from the account_data pointer to reach the specific field
macro_rules! acc_data_dec {
    ($field:expr) => {
        core::mem::size_of::<crate::RawAccount>() - core::mem::offset_of!(crate::RawAccount, $field)
    };
}

macro_rules! decl_decs {
    // recursive-case 1: matching enum variant
    (
        ($CONST:ident, $field:expr)
        $(, $($tail:tt)*)?
    ) => {
        pub(crate) const $CONST: usize = acc_data_dec!($field);

        decl_decs!(
            $($($tail)*)?
        );
    };

    () => {};
}

decl_decs!(
    (IS_SIGNER_DEC, is_signer),
    (IS_WRITABLE_DEC, is_writable),
    (IS_EXECUTABLE_DEC, is_executable),
    (REALLOC_BUDGET_USED_DEC, realloc_budget_used),
    (KEY_DEC, key),
    (OWNER_DEC, owner),
    (LAMPORTS_DEC, lamports),
    (DATA_LEN_DEC, data_len),
);
