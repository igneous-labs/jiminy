use core::{iter::FusedIterator, marker::PhantomData, ptr::null_mut};

use crate::{Account, Accounts, MAX_TX_ACCOUNTS, NON_DUP_MARKER};

#[derive(Debug)]
enum DeserAccount<'account> {
    NonDup(Account<'account>),
    Dup(usize),
}

#[derive(Debug)]
enum AccountsDeserItem<'account> {
    Account(DeserAccount<'account>),
    End(*mut u8),
}

/// Account deserializer that discards deserialized accounts
#[derive(Debug)]
struct AccountsDeser<'account> {
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

    #[inline]
    pub const fn itrs_rem(&self) -> usize {
        self.remaining_accounts + 1
    }
}

impl<'account> Iterator for AccountsDeser<'account> {
    type Item = AccountsDeserItem<'account>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            return None;
        }

        if self.remaining_accounts == 0 {
            self.curr = null_mut();
            return Some(AccountsDeserItem::End(self.curr));
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

        Some(AccountsDeserItem::Account(acc))
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let (curr, accum) =
            (0..self.remaining_accounts).fold((self.curr, init), |(curr, accum), _| {
                let (new_curr, acc) = if unsafe { *curr } == NON_DUP_MARKER {
                    let (new_curr, acc) = unsafe { Account::non_dup_from_ptr(curr) };
                    (new_curr, DeserAccount::NonDup(acc))
                } else {
                    let (new_curr, acc) = unsafe { Account::dup_from_ptr(curr) };
                    (new_curr, DeserAccount::Dup(acc))
                };
                (new_curr, f(accum, AccountsDeserItem::Account(acc)))
            });
        f(accum, AccountsDeserItem::End(curr))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.itrs_rem(), Some(self.itrs_rem()))
    }
}

impl ExactSizeIterator for AccountsDeser<'_> {}

impl FusedIterator for AccountsDeser<'_> {}

/// Populated fixed size [`Accounts`]
/// that can then be output at the end of deserialization
#[derive(Debug)]
pub struct CompletedAccountsDeser<'account, const MAX_ACCOUNTS: usize = MAX_TX_ACCOUNTS> {
    /// Populated from deserializing the whole accounts segment of the entrypoint.
    ///
    /// If number of accounts > `MAX_ACCOUNTS`, those accounts are simply discarded and lost
    pub accounts: Accounts<'account, MAX_ACCOUNTS>,

    /// Pointer to next segment of runtime-serialized entrypoint data (instruction data)
    pub next: *mut u8,
}

impl<const MAX_ACCOUNTS: usize> CompletedAccountsDeser<'_, MAX_ACCOUNTS> {
    /// # Safety
    /// - input must be pointer to start of runtime-serialized entrypoint data
    #[inline]
    pub unsafe fn deser(input: *mut u8) -> Self {
        let total_accounts: &[u8; 8] = &*input.cast();
        let total_accounts = u64::from_le_bytes(*total_accounts) as usize;
        let input = input.add(8);
        AccountsDeser::new(input, total_accounts).collect()
    }
}

impl<'account, const MAX_ACCOUNTS: usize> FromIterator<AccountsDeserItem<'account>>
    for CompletedAccountsDeser<'account, MAX_ACCOUNTS>
{
    #[inline]
    fn from_iter<T: IntoIterator<Item = AccountsDeserItem<'account>>>(iter: T) -> Self {
        iter.into_iter().fold(
            CompletedAccountsDeser {
                accounts: Accounts::new(),
                next: null_mut(),
            },
            |mut this, next| {
                match next {
                    AccountsDeserItem::Account(acc) => {
                        let acc = match acc {
                            DeserAccount::NonDup(a) => a,
                            DeserAccount::Dup(idx) => unsafe {
                                // bitwise copy of the &UnsafeCell<[u8]>.
                                //
                                // slice::get_unchecked safety: runtime should always return indices
                                // that we've already deserialized, which is < len()
                                this.accounts.accounts.get_unchecked(idx).assume_init_read()
                            },
                        };
                        let _maybe_discarded: Result<_, _> = this.accounts.push(acc);
                    }
                    AccountsDeserItem::End(ptr) => {
                        this.next = ptr;
                    }
                };
                this
            },
        )
    }
}
