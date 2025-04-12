use core::{cmp::min, marker::PhantomData, mem::size_of, ptr::NonNull};

use crate::{
    Account, AccountRaw, Accounts, BPF_ALIGN_OF_U128, MAX_PERMITTED_DATA_INCREASE, NON_DUP_MARKER,
};

/// # Returns
/// `(pointer to start of instruction data, saved deserialized accounts)`.
///
/// If the number of accounts exceeds the capacity of Accounts, the accounts that come
/// later are discarded.
///
/// # Safety
/// - `input` must point to start of runtime serialized buffer
#[inline]
pub unsafe fn deser_accounts<'account, const MAX_ACCOUNTS: usize>(
    input: *mut u8,
) -> (*mut u8, Accounts<'account, MAX_ACCOUNTS>) {
    let accounts_len_buf = &*input.cast();
    let accounts_len = u64::from_le_bytes(*accounts_len_buf) as usize;
    let mut input = input.add(8);

    let mut res = Accounts::new();

    let saved_accounts_len = min(accounts_len, MAX_ACCOUNTS);

    for _ in 0..saved_accounts_len {
        let (new_input, acc) = match input.read() {
            NON_DUP_MARKER => Account::non_dup_from_ptr(input),
            _dup => {
                let (new_input, idx) = Account::dup_from_ptr(input);
                // bitwise copy of pointer
                //
                // slice::get_unchecked safety: runtime should always return indices
                // that we've already deserialized, which is < len()
                let acc = res.accounts.get_unchecked(idx).assume_init_read();
                (new_input, acc)
            }
        };
        // push_unchecked safety: saved_accounts_len bounds check above
        res.push_unchecked(acc);
        input = new_input;
    }

    // some duplicate logic here but avoiding the bounds check branch
    // results in halved CUs per account
    for _ in saved_accounts_len..accounts_len {
        input = match input.read() {
            NON_DUP_MARKER => Account::non_dup_from_ptr(input).0,
            _dup => Account::dup_from_ptr(input).0,
        };
    }

    (input, res)
}

/// Runtime deserialization internals
impl Account<'_> {
    /// Returns (pointer to start of next account or instruction data if last account, deserialized account)
    ///
    /// # Safety
    /// - ptr must be pointing to the start of a non-duplicate account
    ///   in the runtime serialized buffer
    #[inline]
    pub(crate) unsafe fn non_dup_from_ptr(ptr: *mut u8) -> (*mut u8, Self) {
        let inner: NonNull<AccountRaw> = NonNull::new_unchecked(ptr.cast());
        let total_len = size_of::<AccountRaw>()
            + inner.as_ref().data_len as usize
            + MAX_PERMITTED_DATA_INCREASE;

        let res = Self {
            ptr: inner,
            _phantom: PhantomData,
        };
        let ptr = ptr.add(total_len);
        let ptr = ptr.add(ptr.align_offset(BPF_ALIGN_OF_U128));
        let ptr = ptr.add(8);

        (ptr, res)
    }

    /// Returns (pointer to start of next account or instruction data if last account, index of duplicated account)
    ///
    /// # Safety
    /// - ptr must be pointing to the start of a duplicate account in the runtime serialized buffer
    #[inline]
    unsafe fn dup_from_ptr(ptr: *mut u8) -> (*mut u8, usize) {
        let idx: &[u8; 8] = &*ptr.cast();
        let idx = u64::from_le_bytes(*idx) as usize;
        (ptr.add(8), idx)
    }
}
