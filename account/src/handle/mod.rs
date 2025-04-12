//! Why not just `impl Deref<[Account]>` for [`Accounts`], have entrypoint take an `&mut [Account]` arg,
//! then work with array indices as handles instead of creating your own handle system?
//!
//! - We want to tightly control accesses to underlying [`Account`]s to avoid UB. `slice` is a bit too permissive:
//!   e.g. [`primitive::slice::split_at_mut`] might give 2 mutable slices where an element in both subslices point to the same
//!   underlying Account due to solana runtime duplication, resulting in the possibility of simultaneous mutable borrow UB

mod accounts;

use core::marker::PhantomData;

pub use accounts::*;

use crate::Account;

/// An opaque handle to an [`Account`] owned by an [`Accounts`].
///
/// This abstraction allows us to hold these handles across
/// `&mut` accesses to [`Accounts`] e.g. during CPIs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AccountHandle<'account> {
    idx: u8,

    /// Bounding lifetime by [`Account`]'s lifetime ensures
    /// at compile time the underlying [`Account`] data is valid for all usages of this struct
    _account_lifetime: PhantomData<Account<'account>>,
}
