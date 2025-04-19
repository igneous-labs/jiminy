//! Why not just `impl Deref<[Account]>` for [`Accounts`], have entrypoint take an `&mut [Account]` arg,
//! then work with array indices as handles instead of creating your own handle system?
//!
//! - We want to tightly control accesses to underlying [`Account`]s to avoid UB. `slice` is a bit too permissive:
//!   e.g. [`primitive::slice::split_at_mut`] might give 2 mutable slices where an element in both subslices point to the same
//!   underlying Account due to solana runtime duplication, resulting in the possibility of simultaneous mutable borrow UB

mod accounts;

use core::{marker::PhantomData, ptr::NonNull};

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
/// # Implementation details
///
/// - this is just a thin wrapper around `NonNull<Account>` to introduce the 'account lifetime and invariance
/// - the `'account` lifetime is pretty much synonymous with `'static` since the buffer it points to is valid for the entire
///   program's execution
#[derive(Debug, Clone, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AccountHandle<'account> {
    pub(crate) ptr: NonNull<Account>,

    // Need this to remove covariance of NonNull;
    // all `Account`s must have the same 'account lifetime.
    //
    // TBH I dont fully get it either yet but this thing is like
    // an UnsafeCell so we should follow UnsafeCell's variance
    // https://doc.rust-lang.org/nomicon/subtyping.html#variance
    pub(crate) _phantom: PhantomData<&'account mut Account>,
}

/// Pointer equality, tells you if 2 different AccountHandles
/// point to the same underlying (duplicated) Account.
impl PartialEq for AccountHandle<'_> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }
}

impl Eq for AccountHandle<'_> {}
