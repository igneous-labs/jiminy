//! TODO: other instructions
//!
//! TODO: the `generic_array_struct` structs should be part of core crate
//! portable across different environments (client-side, wasm etc) instead

use core::{array, iter::Zip};
use jiminy_cpi::{account::AccountHandle, AccountPerms};

mod assign;
mod create_account;
mod internal_utils;
mod transfer;

pub use assign::*;
pub use create_account::*;
pub use transfer::*;

pub type AccountHandlePerms<'account, const ACCOUNTS: usize> = Zip<
    array::IntoIter<AccountHandle<'account>, ACCOUNTS>,
    array::IntoIter<AccountPerms, ACCOUNTS>,
>;
