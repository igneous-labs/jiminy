//! Why not just `impl Deref<[Account]>` for [`Accounts`], have entrypoint take an `&mut [Account]` arg,
//! then work with array indices as handles instead of creating your own handle system?
//!
//! - We want to tightly control accesses to underlying [`Account`]s to avoid UB. `slice` is a bit too permissive:
//!   e.g. [`primitive::slice::split_at_mut`] might give 2 mutable slices where an element in both subslices point to the same
//!   underlying Account due to solana runtime duplication, resulting in the possibility of simultaneous mutable borrow UB

mod accounts;

use core::{hash::Hash, marker::PhantomData, ptr::NonNull};

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
/// - this is just a thin wrapper around `NonNull<Account>` to introduce the `'account` lifetime and invariance
/// - the `'account` lifetime spans from when the [`Accounts`] struct is deserialized at the start
///   of the program to when it is dropped on program exit. This is usually `'static`.
///
/// # Note on core traits
///
/// `Ord`, `Eq` and `Hash` are derived, meaning they use the operations for [`NonNull`], which is
/// just comparison of the raw pointer value. This is what we want since
/// **pointer equality <-> handle refers to the same (maybe duplicated) account**
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccountHandle<'account> {
    pub(crate) ptr: NonNull<Account>,

    /// Need this to remove covariance of NonNull;
    /// all [`AccountHandle`]s must have the same `'account` lifetime.
    ///
    /// This prevents stuff like:
    /// - user creating a fake [`Account`] on the stack, creating an [`AccountHandle`]
    ///   to it with a shorter lifetime and then passing it to [`Accounts::get`]
    ///
    /// TBH I dont fully get it either yet but this thing is like
    /// an `UnsafeCell` so we should follow `UnsafeCell`'s variance
    /// https://doc.rust-lang.org/nomicon/subtyping.html#variance
    pub(crate) _phantom: PhantomData<&'account mut Account>,
}
