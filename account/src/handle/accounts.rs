use core::{mem::MaybeUninit, slice};

use jiminy_program_error::ProgramError;

use crate::{Account, AccountHandle, MAX_TX_ACCOUNTS};

// NB: MAX_ACCOUNTS should be able to fit into a u8, but its actually
// usually more CU efficient to use usize or u32 because ebpf only has
// 32-bit and 64-bit ALUs, so any ops with u8 might be preceded with/succeeded by
// ops like zeroing out the high bits of the register etc

/// An opaque sequence of accounts passed to the instruction by the runtime.
///
/// It dispenses [`AccountHandle`]s that then allow consumers to borrow [`Account`]s
/// either mutably or immutably in a safe way
///
/// `MAX_TX_ACCOUNTS` is max capacity of accounts, must be <= 255
#[derive(Debug)]
pub struct Accounts<'account, const MAX_ACCOUNTS: usize = MAX_TX_ACCOUNTS> {
    pub(crate) accounts: [MaybeUninit<AccountHandle<'account>>; MAX_ACCOUNTS],
    pub(crate) len: usize,
}

/// Accessors
impl<'account, const MAX_ACCOUNTS: usize> Accounts<'account, MAX_ACCOUNTS> {
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// # Safety
    /// - idx should be within bounds and point to an initialized AccountHandle
    ///
    /// # Panics
    /// - if idx out of bounds of capacity
    #[inline(always)]
    pub const unsafe fn handle_unchecked(&self, idx: usize) -> AccountHandle<'account> {
        self.accounts[idx].assume_init()
    }

    #[inline(always)]
    pub const fn handle(&self, idx: usize) -> Option<AccountHandle<'account>> {
        if self.len() <= idx {
            None
        } else {
            Some(unsafe { self.handle_unchecked(idx) })
        }
    }

    #[inline(always)]
    pub const fn get(&self, handle: AccountHandle) -> &Account {
        // safety: handle should be a valid handle previously
        // dispensed by `get_handle` or `get_handle_unchecked`,
        // so it should point to a valid Account
        unsafe { handle.ptr.as_ref() }
    }

    /// Only 1 account in `Self` can be mutated at any time due to the presence of
    /// duplication markers in the runtime.
    #[inline(always)]
    pub fn get_mut(&mut self, mut handle: AccountHandle) -> &mut Account {
        // safety: handle should be a valid handle previously
        // dispensed by `handle` or `handle_unchecked`,
        // so it should point to a valid Account.
        //
        // we have exclusive (mut) access to self here,
        // so its safe to return &mut Account
        unsafe { handle.ptr.as_mut() }
    }
}

/// Iter
impl<'account, const MAX_ACCOUNTS: usize> Accounts<'account, MAX_ACCOUNTS> {
    pub const fn as_slice(&self) -> &[AccountHandle<'account>] {
        unsafe { slice::from_raw_parts(self.accounts.as_ptr().cast(), self.len()) }
    }

    // do not make as_mut_slice() because the array of account pointers
    // should not be mutable
}

/// Convenience methods for common operations
impl<const MAX_ACCOUNTS: usize> Accounts<'_, MAX_ACCOUNTS> {
    /// Transfers lamports from one account to the other by
    /// directly decrementing from's and incrementing to's.
    ///
    /// Does nothing if `from == to`, but still performs the checks
    #[inline(always)]
    pub fn transfer_direct(
        &mut self,
        from: AccountHandle,
        to: AccountHandle,
        lamports: u64,
    ) -> Result<(), ProgramError> {
        self.get_mut(from).dec_lamports(lamports)?;
        self.get_mut(to).inc_lamports(lamports)
    }

    /// See [`Self::transfer_direct`].
    ///
    /// # Safety
    /// - rules of [`Account::dec_lamports_unchecked`] apply
    /// - rules of [`Account::inc_lamports_unchecked`] apply
    #[inline(always)]
    pub unsafe fn transfer_direct_unchecked(
        &mut self,
        from: AccountHandle,
        to: AccountHandle,
        lamports: u64,
    ) {
        self.get_mut(from).dec_lamports_unchecked(lamports);
        self.get_mut(to).inc_lamports_unchecked(lamports);
    }

    /// Close an account by
    ///
    /// 1. realloc to 0 size
    /// 2. assign to system program
    /// 3. [`Self::transfer_direct`] all lamports away to `refund_rent_to`
    ///
    /// Account will still exist with same balance but with
    /// zero sized data and owner = system program
    /// if `close == refund_rent_to`
    #[inline(always)]
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
}
