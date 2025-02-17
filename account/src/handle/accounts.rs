use core::{marker::PhantomData, mem::MaybeUninit};

use jiminy_program_error::ProgramError;

use crate::{Account, MAX_TX_ACCOUNTS};

use super::AccountHandle;

/// An opaque sequence of accounts passed to the instruction by the runtime.
///
/// It dispenses [`AccountHandle`]s that then allow consumers to borrow [`Account`]s
/// either mutably or immutably in a safe way
///
/// `MAX_TX_ACCOUNTS` is max capacity of accounts, must be <= 255
#[derive(Debug)]
pub struct Accounts<'account, const MAX_ACCOUNTS: usize = MAX_TX_ACCOUNTS> {
    pub(crate) accounts: [MaybeUninit<Account<'account>>; MAX_ACCOUNTS],
    len: u8,
}

/// Construction
impl<'account, const MAX_ACCOUNTS: usize> Accounts<'account, MAX_ACCOUNTS> {
    #[inline]
    pub const fn new() -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const UNINIT: MaybeUninit<Account<'_>> = MaybeUninit::uninit();

        Self {
            accounts: [UNINIT; MAX_ACCOUNTS],
            len: 0,
        }
    }

    /// # Safety
    /// - [`self`] must not be full (self.len == N)
    #[inline]
    pub unsafe fn push_unchecked(&mut self, account: Account<'account>) {
        let curr_len = self.len();
        self.accounts.get_unchecked_mut(curr_len).write(account);
        self.len += 1;
    }

    /// Returns the account that failed to be pushed into the collection if [`self`] is full.
    #[inline]
    pub fn push(&mut self, account: Account<'account>) -> Result<(), Account<'account>> {
        if self.is_full() {
            Err(account)
        } else {
            unsafe {
                self.push_unchecked(account);
            }
            Ok(())
        }
    }
}

/// Accessors
impl<'account, const MAX_ACCOUNTS: usize> Accounts<'account, MAX_ACCOUNTS> {
    #[inline]
    pub const fn len_u8(&self) -> u8 {
        self.len
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        self.len() >= MAX_ACCOUNTS
    }

    /// # Safety
    /// - idx should be within bounds
    #[inline]
    pub const unsafe fn handle_unchecked(&self, idx: u8) -> AccountHandle<'account> {
        AccountHandle {
            idx,
            _account_lifetime: PhantomData,
        }
    }

    #[inline]
    pub const fn handle(&self, idx: u8) -> Option<AccountHandle<'account>> {
        if self.len_u8() <= idx {
            None
        } else {
            Some(unsafe { self.handle_unchecked(idx) })
        }
    }

    #[inline]
    pub fn get(&self, handle: AccountHandle) -> &Account {
        // safety: handle should be a valid handle previously
        // dispensed by `get_handle` or `get_handle_unchecked`
        unsafe {
            self.accounts
                .get_unchecked(usize::from(handle.idx))
                .assume_init_ref()
        }
    }

    /// Only 1 account in `Self` can be mutated at any time due to the presence of
    /// duplication markers in the runtime.
    ///
    /// Special runtime-specific account mutators defined below are able to work around this limitation
    #[inline]
    pub fn get_mut<'a>(&'a mut self, handle: AccountHandle) -> &'a mut Account<'account> {
        // safety: handle should be a valid handle previously
        // dispensed by `handle` or `handle_unchecked`
        unsafe {
            self.accounts
                .get_unchecked_mut(usize::from(handle.idx))
                .assume_init_mut()
        }
    }

    #[inline]
    pub const fn iter<'a>(&'a self) -> AccountsHandleIter<'a, 'account> {
        AccountsHandleIter {
            head: 0,
            tail: self.len_u8(),
            _accounts: PhantomData,
        }
    }
}

/// Special runtime-specific account mutators that require simultaneous mut access
/// to 2 or more accounts
impl<const MAX_ACCOUNTS: usize> Accounts<'_, MAX_ACCOUNTS> {
    /// No-op if `from == to`
    #[inline]
    pub fn transfer_direct(
        &mut self,
        from: AccountHandle,
        to: AccountHandle,
        lamports: u64,
    ) -> Result<(), ProgramError> {
        self.get_mut(from).dec_lamports(lamports)?;
        self.get_mut(to).inc_lamports(lamports)
    }

    /// No-op if `from == to`
    ///
    /// # Safety
    /// - rules of [`Account::dec_lamports_unchecked`] apply
    /// - rules of [`Account::inc_lamports_unchecked`] apply
    #[inline]
    pub unsafe fn transfer_direct_unchecked(
        &mut self,
        from: AccountHandle,
        to: AccountHandle,
        lamports: u64,
    ) {
        self.get_mut(from).dec_lamports_unchecked(lamports);
        self.get_mut(to).inc_lamports_unchecked(lamports);
    }

    /// Account will still exist with same balance but with
    /// zero sized data and owner = system program
    /// if `close == refund_rent_to`
    #[inline]
    pub fn close(
        &mut self,
        close: AccountHandle,
        refund_rent_to: AccountHandle,
    ) -> Result<(), ProgramError> {
        let close_acc = self.get_mut(close);
        close_acc.realloc(0, false)?;
        close_acc.assign_direct([0u8; 32]); // TODO: use const pubkey for system program
        let balance = close_acc.lamports();
        self.transfer_direct(close, refund_rent_to, balance)
    }

    /// Account will still exist with same balance but with
    /// zero sized data and owner = system program
    /// if `close == refund_rent_to`
    ///
    /// # Safety
    /// - rules for [`Account::realloc_unchecked`] apply
    /// - rules for [`Self::transfer_direct_unchecked`] apply
    #[inline]
    pub unsafe fn close_unchecked(&mut self, close: AccountHandle, refund_rent_to: AccountHandle) {
        let close_acc = self.get_mut(close);
        close_acc.realloc_unchecked(0);
        close_acc.assign_direct([0u8; 32]); // TODO: use const pubkey for system program
        let balance = close_acc.lamports();
        self.transfer_direct_unchecked(close, refund_rent_to, balance)
    }
}

impl<const MAX_ACCOUNTS: usize> Default for Accounts<'_, MAX_ACCOUNTS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over an [`Accounts`]' [`AccountHandle`]s
pub struct AccountsHandleIter<'a, 'account> {
    head: u8,
    tail: u8,
    /// we don't actually need to hold the `Accounts` ref since we're just returning u8s,
    /// but we must bound this struct's lifetimes to the ref's lifetimes.
    ///
    /// We can also remove the const generic
    _accounts: PhantomData<&'a Account<'account>>,
}

impl<'account> Iterator for AccountsHandleIter<'_, 'account> {
    type Item = AccountHandle<'account>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.head == self.tail {
            None
        } else {
            let res = AccountHandle {
                idx: self.head,
                _account_lifetime: PhantomData,
            };
            self.head += 1;
            Some(res)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = (self.tail - self.head).into();
        (rem, Some(rem))
    }
}

impl DoubleEndedIterator for AccountsHandleIter<'_, '_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.head == self.tail {
            None
        } else {
            self.tail -= 1;
            Some(AccountHandle {
                idx: self.tail,
                _account_lifetime: PhantomData,
            })
        }
    }
}

impl ExactSizeIterator for AccountsHandleIter<'_, '_> {}
