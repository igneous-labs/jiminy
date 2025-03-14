use core::{cell::UnsafeCell, iter::FusedIterator, marker::PhantomData};

use crate::{Account, BPF_ALIGN_OF_U128, MAX_PERMITTED_DATA_INCREASE, NON_DUP_MARKER};

mod saving;

pub use saving::*;

#[derive(Debug)]
pub enum DeserAccount<'account> {
    NonDup(Account<'account>),
    Dup(usize),
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
        let data_len_slice: &[u8; 8] = &*ptr.add(Self::DATA_LEN_OFFSET).cast();
        let data_len = u64::from_le_bytes(*data_len_slice);
        let total_len = Self::HEADER_LEN + data_len as usize + MAX_PERMITTED_DATA_INCREASE;

        let res = Self(
            &*(core::ptr::slice_from_raw_parts(ptr.cast_const(), total_len)
                as *const UnsafeCell<[u8]>),
        );
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
    pub(crate) unsafe fn dup_from_ptr(ptr: *mut u8) -> (*mut u8, usize) {
        let idx: &[u8; 8] = &*ptr.cast();
        let idx = u64::from_le_bytes(*idx) as usize;
        (ptr.add(8), idx)
    }
}

/// Account deserializer that discards deserialized accounts
#[derive(Debug)]
pub struct AccountsDeser<'account> {
    curr: *mut u8,
    remaining_accounts: usize,
    _accounts_lifetime: PhantomData<Account<'account>>,
}

impl AccountsDeser<'_> {
    /// # Safety
    /// - ptr must point to start of an account in the
    ///   accounts segment of the memory block serialized by the runtime
    #[inline]
    pub const unsafe fn new(curr: *mut u8, remaining_accounts: usize) -> Self {
        Self {
            curr,
            remaining_accounts,
            _accounts_lifetime: PhantomData,
        }
    }

    /// Returns Ok(pointer to start of instruction data) if completed,
    /// Err(self) otherwise
    #[inline]
    pub const fn finish(self) -> Result<*mut u8, Self> {
        if self.remaining_accounts == 0 {
            Ok(unsafe { self.finish_unchecked() })
        } else {
            Err(self)
        }
    }

    /// # Safety
    /// - account deserialization must have completed, or else the returned pointer
    ///   does not point to valid instruction data
    #[inline]
    pub const unsafe fn finish_unchecked(self) -> *mut u8 {
        self.curr
    }
}

impl<'account> Iterator for AccountsDeser<'account> {
    type Item = DeserAccount<'account>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_accounts == 0 {
            return None;
        }

        let (new_curr, acc) = if unsafe { *self.curr } == NON_DUP_MARKER {
            let (new_curr, acc) = unsafe { Account::non_dup_from_ptr(self.curr) };
            (new_curr, DeserAccount::NonDup(acc))
        } else {
            let (new_curr, acc) = unsafe { Account::dup_from_ptr(self.curr) };
            (new_curr, DeserAccount::Dup(acc))
        };

        self.curr = new_curr;
        self.remaining_accounts -= 1;

        Some(acc)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining_accounts, Some(self.remaining_accounts))
    }
}

impl ExactSizeIterator for AccountsDeser<'_> {}

impl FusedIterator for AccountsDeser<'_> {}
