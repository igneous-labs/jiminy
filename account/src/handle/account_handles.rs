use core::{mem::MaybeUninit, ops::Deref};

use crate::MAX_TX_ACCOUNTS;

use super::AccountHandle;

/// A slice of [`AccountHandle`]s
///
/// `MAX_ACCOUNTS` is max capacity of accounts, must be <= 255
#[derive(Debug, Clone, Copy)]
pub struct AccountHandles<'account, const MAX_ACCOUNTS: usize = MAX_TX_ACCOUNTS> {
    accounts: [MaybeUninit<AccountHandle<'account>>; MAX_ACCOUNTS],
    len: u8,
}

impl<'account, const MAX_ACCOUNTS: usize> AccountHandles<'account, MAX_ACCOUNTS> {
    #[inline]
    pub const fn new() -> Self {
        const UNINIT: MaybeUninit<AccountHandle<'_>> = MaybeUninit::uninit();
        Self {
            accounts: [UNINIT; MAX_ACCOUNTS],
            len: 0,
        }
    }

    /// # Safety
    /// - [`self`] must not be full (self.len() == N)
    #[inline]
    pub unsafe fn push_unchecked(&mut self, handle: AccountHandle<'account>) {
        let curr_len = self.len();
        self.accounts.get_unchecked_mut(curr_len).write(handle);
        self.len += 1;
    }

    /// Returns the handle that failed to be pushed into the collection if [`self`] is full.
    #[inline]
    pub fn push(&mut self, handle: AccountHandle<'account>) -> Result<(), AccountHandle<'account>> {
        if self.is_full() {
            Err(handle)
        } else {
            unsafe {
                self.push_unchecked(handle);
            }
            Ok(())
        }
    }

    #[inline]
    pub const fn as_slice(&self) -> &[AccountHandle<'account>] {
        unsafe { core::slice::from_raw_parts(self.accounts.as_ptr().cast(), self.len()) }
    }

    #[inline]
    pub const fn len_u8(&self) -> u8 {
        self.len
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len_u8() as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len_u8() == 0
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        self.len() == MAX_ACCOUNTS
    }
}

impl<'account, const MAX_ACCOUNTS: usize> Deref for AccountHandles<'account, MAX_ACCOUNTS> {
    type Target = [AccountHandle<'account>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// Discards AccountHandle if iterator yields more items than `N`
impl<'account, const MAX_ACCOUNTS: usize> FromIterator<AccountHandle<'account>>
    for AccountHandles<'account, MAX_ACCOUNTS>
{
    #[inline]
    fn from_iter<T: IntoIterator<Item = AccountHandle<'account>>>(iter: T) -> Self {
        let mut res = Self::new();
        let iter = iter.into_iter();
        for handle in iter {
            let _maybe_discarded: Result<(), AccountHandle<'account>> = res.push(handle);
        }
        res
    }
}

impl<'a, 'account, const MAX_ACCOUNTS: usize> IntoIterator
    for &'a AccountHandles<'account, MAX_ACCOUNTS>
{
    // change this to references instead and remove copied if size of AccountHandle > word size
    type Item = AccountHandle<'account>;

    type IntoIter = core::iter::Copied<core::slice::Iter<'a, Self::Item>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter().copied()
    }
}

impl<const MAX_ACCOUNTS: usize> Default for AccountHandles<'_, MAX_ACCOUNTS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
