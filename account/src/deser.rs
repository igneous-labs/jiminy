// Implementation notes:
//
// - When working with raw pointers, rust cannot enforce aliasing rules, so it cannot optimize
//   away redundant reads, so always try to reuse already computed offset data.
//   E.g. there used to be an AccountHandle::dup_from_ptr method for API symmetry with non_dup_from_ptr,
//   but that resulted in a redundant read of the duplicate marker vs if we just used the matched byte directly.
// - #[inline(always)] for all fns here replaced with #[inline]. Caused instructions test program to -4 CUs but +16 binsize

use core::{cell::UnsafeCell, cmp::min, mem::MaybeUninit};

use crate::{
    Account, AccountHandle, Accounts, BPF_ALIGN_OF_U128, MAX_PERMITTED_DATA_INCREASE,
    NON_DUP_MARKER,
};

/// # Returns
/// `(pointer to start of instruction data, saved deserialized accounts)`.
///
/// If the number of accounts exceeds the capacity of Accounts, the accounts that come
/// later are discarded.
///
/// # Safety
/// - `input` must point to start of runtime serialized buffer
///
/// # Notes
/// - `_scope` is just an unused param that is meant to bound the
///   `'account` lifetime; the returned [`Accounts`] will have the same
///   lifetime as `_scope`
#[inline]
pub unsafe fn deser_accounts<const MAX_ACCOUNTS: usize>(
    _scope: &(),
    input: *mut u8,
) -> (*mut u8, Accounts<'_, MAX_ACCOUNTS>) {
    // this is uninit, interior mutable const shouldnt affect it
    #[allow(clippy::declare_interior_mutable_const)]
    const UNINIT: MaybeUninit<AccountHandle<'_>> = MaybeUninit::uninit();

    // cast-safety: 0x40... is 8-byte aligned
    let accounts_len = input.cast::<u64>().read() as usize;
    let input = input.add(8);

    let saved_accounts_len = min(accounts_len, MAX_ACCOUNTS);
    let mut accounts = [UNINIT; MAX_ACCOUNTS];

    // Aside: ive tried everything:
    // - rewriting fold as for loop
    // - using mutation instead of reassigning input
    // - deleting non_dup_from_ptr() and manually inlining it instead
    // - even the redacted `input: &mut *mut u8`
    //
    // but somehow the compiler insists on doing the absolutely trashcan thing of
    // using 2 registers to store and manipulate input pointer, one for current pointer and
    // one for `current pointer - 8` for some goddman reason, resulting
    // in 2 more instructions than pinocchio in the minimal case.

    // its probably more functional to have `accounts` as part of the
    // accumulator value but the compiler generates some absolutely
    // disgustingly inefficient code when that happens, so just mutate
    // `accounts` in the closure instead.
    //
    // Probably a good rule of thumb is to make sure fold() accumulator values
    // fit into a single register, so only ints and references allowed
    let input = (0..saved_accounts_len).fold(input, |input, i| {
        let (new_input, acc_handle) = match input.read() {
            NON_DUP_MARKER => AccountHandle::non_dup_from_ptr(input, &accounts),
            dup_idx => AccountHandle::dup_from_ptr(input, dup_idx, &accounts),
        };
        // unchecked index safety: bounds checked by saved_accounts_len above
        accounts.get_unchecked_mut(i).write(acc_handle);
        new_input
    });

    // some duplicate logic here but this avoid bounds check before pushing
    // into accounts. Results in reduced CUs per account
    let input = (saved_accounts_len..accounts_len).fold(input, |input, _| match input.read() {
        NON_DUP_MARKER => AccountHandle::non_dup_from_ptr(input, &accounts).0,
        dup_idx => AccountHandle::dup_from_ptr(input, dup_idx, &accounts).0,
    });

    (
        input,
        // we use to have a nice `push_unchecked()` method for adding accounts
        // to `Accounts` that would write then inc length instead of constructing it at the end like here
        // but the compiler couldnt figure out that it could accumulate the final length
        // and only set it at the end so it was doing the absolutely redacted thing of
        // load-increment-store (3x the instructions!!!) on every iteration of deserializing an account.
        Accounts {
            accounts,
            len: saved_accounts_len,
        },
    )
}

/// Runtime deserialization internals
impl<'account> AccountHandle<'account> {
    /// Returns (pointer to start of next account or instruction data if last account, deserialized account).
    ///
    /// # Safety
    /// - ptr must be pointing to the start of a non-duplicate account
    ///   in the runtime serialized buffer
    #[inline]
    pub(crate) unsafe fn non_dup_from_ptr(
        ptr: *mut u8,
        _accounts: &[MaybeUninit<AccountHandle<'account>>], // here just to bound lifetimes
    ) -> (*mut u8, Self) {
        let inner: *mut Account = ptr.cast();
        let total_len =
            core::mem::size_of::<Account>() + (*inner).data_len() + MAX_PERMITTED_DATA_INCREASE;

        let res = Self {
            account: &*inner.cast::<UnsafeCell<Account>>(),
        };
        let ptr = ptr.add(total_len);
        let ptr = ptr.add(ptr.align_offset(BPF_ALIGN_OF_U128));
        let ptr = ptr.add(8);

        (ptr, res)
    }

    /// Factored out into its own fn so that we can stick #[cold] on it
    #[cold]
    #[inline]
    unsafe fn dup_from_ptr(
        ptr: *mut u8,
        dup_idx: u8,
        accounts: &[MaybeUninit<AccountHandle<'account>>],
    ) -> (*mut u8, Self) {
        // bitwise copy of pointer
        //
        // slice::get_unchecked safety: runtime should always return indices
        // that we've already deserialized, which is within bounds of `accounts`
        let dup_acc_handle = accounts.get_unchecked(dup_idx as usize).assume_init();
        (ptr.add(8), dup_acc_handle)
    }
}
