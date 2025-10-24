/// Maximum number of accounts that a transaction may process.
///
/// This value is used to set the maximum number of accounts that a program
/// is expecting and statically initialize the array of `AccountInfo`.
///
/// This is based on the current [maximum number of accounts] that a transaction
/// may lock in a block.
///
/// [maximum number of accounts]: https://github.com/anza-xyz/agave/blob/2e6ca8c1f62db62c1db7f19c9962d4db43d0d550/runtime/src/bank.rs#L3209-L3221
pub const MAX_TX_ACCOUNTS: usize = 128;

/// Value used to indicate that a serialized account is not a duplicate.
pub const NON_DUP_MARKER: u8 = u8::MAX;

pub const MAX_PERMITTED_DATA_INCREASE: usize = 1_024 * 10;

pub const BPF_ALIGN_OF_U128: usize = 8;

/// 10 MiB
///
/// Copied from agave, same-named const
pub const MAX_PERMITTED_DATA_LENGTH: usize = 10 * 1024 * 1024;
