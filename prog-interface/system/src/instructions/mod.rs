//! TODO: other instructions
//!
//! TODO: the `generic_array_struct` structs should be part of core crate
//! portable across different environments (client-side, wasm etc) instead

mod assign;
mod create_account;
mod internal_utils;
mod transfer;

use core::{
    iter::{Copied, Zip},
    slice,
};

pub use assign::*;
pub use create_account::*;
use jiminy_cpi::{account::AccountHandle, AccountPerms};
pub use transfer::*;

pub type Instruction<'account, 'data> = jiminy_cpi::Instr<
    'account,
    'data,
    Zip<
        Copied<slice::Iter<'data, AccountHandle<'account>>>,
        Copied<slice::Iter<'data, AccountPerms>>,
    >,
>;
