//! TODO: other instructions
//!
//! TODO: the `generic_array_struct` structs should be part of core crate
//! portable across different environments (client-side, wasm etc) instead

mod assign;
mod create_account;
mod internal_utils;
mod transfer;

pub use assign::*;
pub use create_account::*;
pub use transfer::*;

/// WithdrawNonceAccount has the most accounts
const MAX_ACCOUNTS_LEN: usize = 5;

/// `Pubkey::MAX_SEED_LEN`
const MAX_SEED_LEN: usize = 32;

/// CreateAccountWithSeed has longest possible data
const MAX_DATA_LEN: usize = 4 + 32 + MAX_SEED_LEN + 8 + 8 + 32;

pub type Instruction<'account> = jiminy_cpi::Instruction<'account, MAX_DATA_LEN, MAX_ACCOUNTS_LEN>;
