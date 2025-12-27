use core::marker::PhantomData;

use jiminy_program_error::ProgramError;

use crate::{Account, AccountHandle};

/// `Abr` (short for **A**ccount**b**o**r**row) is a program-wide singleton
/// that controls account borrowing at compile-time.
///
/// # Invariants
/// - At most one `Abr` exists for an entire jiminy program. This is guaranteed by
///   inaccessible private field and private constructors. The only way to safely obtain
///   this struct is via [`crate::DeserAccounts::etp_start`]
///   (unless the user chooses to do stupid things with `unsafe`).
///
/// With these invariants, we can now implement rust's XOR borrow rules at compile-time:
/// - `mut` borrow of an account mutable borrows the entire singleton `Abr`
/// - multiple immutable borrows of an account can immutably borrow the singleton `Abr`.
///
/// This invariant also allows us to remove lifetime annotations, since we can assume any
/// `AccountHandle` encountered in a jiminy program is valid
// do not derive Copy or Clone since this is supposed to be a program-wide singleton
#[derive(Debug)]
pub struct Abr {
    _unconstructable_outside_this_crate: PhantomData<()>,
}

impl Abr {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self {
            _unconstructable_outside_this_crate: PhantomData,
        }
    }
}

impl Abr {
    #[inline(always)]
    pub const fn get<'this>(&'this self, handle: AccountHandle<'_>) -> &'this Account {
        // safety: handle should be a valid handle previously
        // dispensed by `get_handle` or `get_handle_unchecked`,
        // so it should point to a valid Account.
        //
        // since we have reference access to self, nothing else
        // should have &mut access to the account
        unsafe { &*handle.account.get() }
    }

    /// Only 1 account can be mutably borrowed at any time due to the presence of
    /// duplication markers in the runtime.
    #[inline(always)]
    pub const fn get_mut<'this>(&'this mut self, handle: AccountHandle<'_>) -> &'this mut Account {
        // safety: handle should be a valid handle previously
        // dispensed by `handle` or `handle_unchecked`,
        // so it should point to a valid Account.
        //
        // we have exclusive (mut) access to self here,
        // nothing else has access to the account,
        // so we can return &mut
        unsafe { &mut *handle.account.get() }
    }

    /// Returns a raw pointer to the underlying Account to avoid UB related to
    /// pointers derived from references. This is currently only used for CPI.
    #[inline(always)]
    pub const fn get_ptr(&self, handle: AccountHandle<'_>) -> *mut Account {
        handle.account.get()
    }

    // TODO: we can now implement simultaneous mutable borrow of two accounts
    // with runtime checks
}

/// Convenience methods for common account operations
impl Abr {
    /// Transfers lamports from one account to the other by
    /// directly decrementing from's and incrementing to's.
    ///
    /// Does nothing if `from == to`, but still performs the checks
    #[inline(always)]
    pub fn transfer_direct<'account>(
        &mut self,
        from: AccountHandle<'account>,
        to: AccountHandle<'account>,
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
    pub unsafe fn transfer_direct_unchecked<'account>(
        &mut self,
        from: AccountHandle<'account>,
        to: AccountHandle<'account>,
        lamports: u64,
    ) {
        self.get_mut(from).dec_lamports_unchecked(lamports);
        self.get_mut(to).inc_lamports_unchecked(lamports);
    }

    /// Close an account owned by the currently executing program by
    ///
    /// 1. [`Self::transfer_direct`] all lamports away to `refund_rent_to`
    /// 2. realloc to 0 size
    /// 3. assign to system program
    ///
    /// Account will still exist with same balance but with
    /// zero sized data and owner = system program
    /// if `close == refund_rent_to`
    #[inline(always)]
    pub fn close<'account>(
        &mut self,
        close: AccountHandle<'account>,
        refund_rent_to: AccountHandle<'account>,
    ) -> Result<(), ProgramError> {
        let balance = self.get(close).lamports();
        self.transfer_direct(close, refund_rent_to, balance)?;
        let close_acc = self.get_mut(close);
        close_acc.realloc(0)?;
        close_acc.assign_direct([0u8; 32]); // TODO: use const pubkey for system program
        Ok(())
    }
}
