//! **Q**: Why not just `impl Deref<[&UnsafeCell<Account>]>` for [`Accounts`], have entrypoint take an `&mut [&UnsafeCell<Account>]` arg,
//! then work with array indices as handles instead of creating your own handle system?
//!
//! **A**: We want to tightly control accesses to underlying [`Account`]s to avoid UB. `slice` is a bit too permissive:
//! e.g. [`primitive::slice::split_at_mut`] might give 2 mutable slices where an element in both subslices point to the same
//! underlying Account due to solana runtime duplication, resulting in the possibility of simultaneous mutable borrow UB

mod accounts;

use core::{
    cell::UnsafeCell,
    cmp::Ordering,
    hash::{Hash, Hasher},
};

pub use accounts::*;

use crate::Account;

/// An opaque handle to an [`Account`] owned by an [`Accounts`].
///
/// This abstraction allows us to hold these handles across
/// `&mut` accesses to [`Accounts`] e.g. during CPIs.
///
/// In general you want to copy this struct out from [`Accounts::as_slice`] instead of
/// using the returned value directly since that would keep [`Accounts`] borrowed.
///
/// To access the underlying account, you must use this handle with [`Accounts::get`] or
/// [`Accounts::get_mut`].
///
/// # Implementation details
///
/// - this is just a thin wrapper around `&UnsafeCell<Account>`
/// - the `'account` lifetime spans from when the [`Accounts`] struct is deserialized at the start
///   of the program to when it is dropped on program exit. This is usually `'static`.
///
/// # Note on core traits
///
/// `Ord`, `Eq` and `Hash` use the operations for comparisons of
/// raw pointer values. This is what we want since
/// **pointer equality <-> handle refers to the same (maybe duplicated) account**
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct AccountHandle<'account> {
    pub(crate) account: &'account UnsafeCell<Account>,
}

impl Ord for AccountHandle<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.account.get().cmp(&other.account.get())
    }
}

impl PartialOrd for AccountHandle<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for AccountHandle<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Equal)
    }
}

impl Eq for AccountHandle<'_> {}

/// Need to hash raw pointer value to maintain
/// `k1 == k2 -> hash(k1) == hash(k2)`
/// invariant
impl Hash for AccountHandle<'_> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.account.get().hash(state);
    }
}
