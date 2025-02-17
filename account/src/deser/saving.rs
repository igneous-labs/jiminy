use crate::{AccountHandle, Accounts, DeserAccount, MAX_TX_ACCOUNTS};

use super::AccountsDeser;

/// Account deserializer that saves deserialized accounts to a fixed size [`Accounts`]
/// that can then be output at the end of deserialization
#[derive(Debug)]
pub struct SavingAccountsDeser<'account, const MAX_ACCOUNTS: usize = MAX_TX_ACCOUNTS> {
    accounts: Accounts<'account, MAX_ACCOUNTS>,
    deser: AccountsDeser<'account>,
}

impl<'account, const MAX_ACCOUNTS: usize> SavingAccountsDeser<'account, MAX_ACCOUNTS> {
    /// # Safety
    /// - ptr must point to start of memory block serialized by the runtime
    #[inline]
    pub const unsafe fn new(ptr: *mut u8, remaining_accounts: usize) -> Self {
        Self {
            accounts: Accounts::new(),
            deser: AccountsDeser::new(ptr, remaining_accounts),
        }
    }

    /// Call after iteration completed to obtain saved [`Accounts`] and [`AccountsDeser`] for any remaining
    /// accounts that can't fit into the [`Accounts`]
    #[inline]
    pub const fn finish(self) -> (AccountsDeser<'account>, Accounts<'account, MAX_ACCOUNTS>) {
        let Self { deser, accounts } = self;
        (deser, accounts)
    }

    #[inline]
    pub const fn itrs_left(&self) -> usize {
        // unchecked arith: len should always <= MAX_ACCOUNTS
        let cap_remaining = MAX_ACCOUNTS - self.accounts.len();
        let remaining_accounts = self.deser.remaining_accounts;
        if cap_remaining > remaining_accounts {
            remaining_accounts
        } else {
            cap_remaining
        }
    }
}

impl<'account, const MAX_ACCOUNTS: usize> Iterator for SavingAccountsDeser<'account, MAX_ACCOUNTS> {
    type Item = AccountHandle<'account>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.accounts.is_full() {
            return None;
        }

        let acc = match self.deser.next()? {
            DeserAccount::NonDup(a) => a,
            DeserAccount::Dup(idx) => unsafe {
                // bitwise copy of the &UnsafeCell<[u8]>
                //
                // slice::get_unchecked safety: runtime should always return indices
                // that we've already deserialized
                self.accounts.accounts.get_unchecked(idx).assume_init_read()
            },
        };

        unsafe {
            // safety: is_full() checked above
            self.accounts.push_unchecked(acc);
            // safety: new account was just pushed
            Some(self.accounts.handle_unchecked(self.accounts.len_u8() - 1))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.itrs_left();
        (rem, Some(rem))
    }
}

impl<const MAX_ACCOUNTS: usize> ExactSizeIterator for SavingAccountsDeser<'_, MAX_ACCOUNTS> {}

const _ASSERT_FITS_ON_STACK: () = {
    if core::mem::size_of::<SavingAccountsDeser>() > 3072 {
        panic!("AccountsDeser size > 3072")
    }
};
