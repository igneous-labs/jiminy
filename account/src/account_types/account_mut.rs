use core::marker::PhantomData;

use jiminy_program_error::{BuiltInProgramError, ProgramError};

use crate::{
    Abr, Account, AccountHandle, DATA_LEN_DEC, LAMPORTS_DEC, MAX_PERMITTED_DATA_INCREASE,
    OWNER_DEC, REALLOC_BUDGET_USED_DEC,
};

/// Only legal way to obtain this is via [`crate::Abr::get_mut`]
// does not impl Clone or Copy - acts like a &mut
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct AccountMut<'a, 'account> {
    pub(crate) handle: AccountHandle<'account>,
    pub(crate) borrow: PhantomData<&'a mut Abr>,
}

impl<'accounts> AccountMut<'_, 'accounts> {
    #[inline(always)]
    pub const fn as_account<'this>(&'this self) -> Account<'this, 'accounts> {
        Account {
            handle: self.handle,
            borrow: PhantomData,
        }
    }
}

/// Accessors
impl<'a> AccountMut<'a, '_> {
    // 'a lifetime: borrow of any field of the account is valid
    // as long as Abr borrow is valid
    #[inline(always)]
    const fn get<T>(&mut self, dec: usize) -> &'a mut T {
        // safety: this is a private internal util for
        // use with well-known fields that have correct alignments
        // and offsets
        unsafe { &mut *self.handle.account_data.sub(dec).cast() }
    }

    /// Only used for CPI helpers.
    ///
    /// To read and manipulate lamports, use
    /// `self.as_account().lamports()`, and
    /// [`Self::set_lamports`], [`Self::inc_lamports`],
    /// [`Self::dec_lamports`] instead.
    #[inline(always)]
    pub const fn lamports_ref_mut(&mut self) -> &'a mut u64 {
        self.get(LAMPORTS_DEC)
    }

    /// Only used for CPI helpers.
    ///
    /// To read and manipulate owner, use
    /// `self.as_account().owner()` and [`Self::assign_direct`] instead.
    #[inline(always)]
    pub const fn owner_mut(&mut self) -> &'a mut [u8; 32] {
        self.get(OWNER_DEC)
    }

    /// Account data is always guaranteed to be 8-byte aligned
    #[inline(always)]
    pub const fn data_mut(&mut self) -> &'a mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.handle.account_data, self.as_account().data_len())
        }
    }
}

/// lamports mutators
impl AccountMut<'_, '_> {
    #[inline(always)]
    pub const fn set_lamports(&mut self, new_lamports: u64) {
        *self.lamports_ref_mut() = new_lamports
    }

    #[inline(always)]
    pub const fn inc_lamports(&mut self, inc_lamports: u64) -> Result<(), ProgramError> {
        match self.as_account().lamports().checked_add(inc_lamports) {
            Some(new_lamports) => {
                self.set_lamports(new_lamports);
                Ok(())
            }
            None => Err(ProgramError::from_builtin(
                BuiltInProgramError::ArithmeticOverflow,
            )),
        }
    }

    /// # Safety
    /// - increment must not result in overflow
    #[inline(always)]
    pub const unsafe fn inc_lamports_unchecked(&mut self, inc_lamports: u64) {
        let new_lamports = self.as_account().lamports() + inc_lamports;
        self.set_lamports(new_lamports);
    }

    #[inline(always)]
    pub const fn dec_lamports(&mut self, dec_lamports: u64) -> Result<(), ProgramError> {
        match self.as_account().lamports().checked_sub(dec_lamports) {
            Some(new_lamports) => {
                self.set_lamports(new_lamports);
                Ok(())
            }
            None => Err(ProgramError::from_builtin(
                BuiltInProgramError::InsufficientFunds,
            )),
        }
    }

    /// # Safety
    /// - decrement must not result in overflow
    #[inline(always)]
    pub const unsafe fn dec_lamports_unchecked(&mut self, dec_lamports: u64) {
        let new_lamports = self.as_account().lamports() - dec_lamports;
        self.set_lamports(new_lamports);
    }
}

/// owner mutators
impl AccountMut<'_, '_> {
    #[inline(always)]
    pub const fn assign_direct(&mut self, new_owner: [u8; 32]) {
        *self.owner_mut() = new_owner;
    }
}

/// realloc
impl AccountMut<'_, '_> {
    #[inline(always)]
    const fn realloc_budget_used_mut(&mut self) -> &mut i32 {
        self.get(REALLOC_BUDGET_USED_DEC)
    }

    #[inline(always)]
    const fn data_len_mut(&mut self) -> &mut u64 {
        self.get(DATA_LEN_DEC)
    }

    #[inline(always)]
    pub fn realloc(&mut self, new_len: usize, zero_init: bool) -> Result<(), ProgramError> {
        // account data lengths should always be <= 10MiB < i32::MAX,
        let curr_len = self.as_account().data_len();
        // `try_from` here is the only thing blocking `const fn`
        let [Ok(new_len_i32), Ok(curr_len_i32)] = [new_len, curr_len].map(i32::try_from) else {
            return Err(ProgramError::from_builtin(
                BuiltInProgramError::InvalidRealloc,
            ));
        };

        // unchecked-arith: all quantities are in [0, 10MiB],
        // these subtractions and additions should never overflow
        let budget_delta = new_len_i32 - curr_len_i32;
        let new_realloc_budget_used = *self.realloc_budget_used_mut() + budget_delta;
        if new_realloc_budget_used > MAX_PERMITTED_DATA_INCREASE as i32 {
            return Err(ProgramError::from_builtin(
                BuiltInProgramError::InvalidRealloc,
            ));
        }

        *self.realloc_budget_used_mut() = new_realloc_budget_used;
        *self.data_len_mut() = new_len as u64;

        if zero_init {
            if let Ok(growth) = usize::try_from(budget_delta) {
                // TODO: see if sol_memset syscall is necessary here,
                // or if ptr::write_bytes is optimized into that
                unsafe {
                    core::ptr::write_bytes(self.handle.account_data.add(curr_len), 0, growth);
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    pub fn shrink_by(&mut self, dec_bytes: usize) -> Result<(), ProgramError> {
        match self.as_account().data_len().checked_sub(dec_bytes) {
            Some(new_len) => self.realloc(new_len, false),
            None => Err(ProgramError::from_builtin(
                BuiltInProgramError::ArithmeticOverflow,
            )),
        }
    }

    #[inline(always)]
    pub fn grow_by(&mut self, inc_bytes: usize, zero_init: bool) -> Result<(), ProgramError> {
        match self.as_account().data_len().checked_add(inc_bytes) {
            Some(new_len) => self.realloc(new_len, zero_init),
            None => Err(ProgramError::from_builtin(
                BuiltInProgramError::ArithmeticOverflow,
            )),
        }
    }
}
