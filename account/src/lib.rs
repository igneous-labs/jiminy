#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

use core::{
    marker::PhantomData,
    mem::{align_of, size_of},
    ptr::NonNull,
};

// Re-exports
pub mod program_error {
    pub use jiminy_program_error::*;
}
use program_error::*;

mod deser;
mod handle;

pub use deser::*;
pub use handle::*;

/// Maximum number of accounts that a transaction may process.
///
/// This value is used to set the maximum number of accounts that a program
/// is expecting and statically initialize the array of `AccountInfo`.
///
/// This is based on the current [maximum number of accounts] that a transaction
/// may lock in a block.
///
/// [maximum number of accounts]: https://github.com/anza-xyz/agave/blob/2e6ca8c1f62db62c1db7f19c9962d4db43d0d550/runtime/src/bank.rs#L3209-L3221
pub const MAX_TX_ACCOUNTS: usize = 128;

/// Value used to indicate that a serialized account is not a duplicate.
pub const NON_DUP_MARKER: u8 = u8::MAX;

pub const MAX_PERMITTED_DATA_INCREASE: usize = 1_024 * 10;

pub const BPF_ALIGN_OF_U128: usize = 8;

/// 10 MiB
///
/// Copied from agave, same-named const
pub const MAX_PERMITTED_DATA_LENGTH: usize = 10 * 1024 * 1024;

/// # Implementation details
///
/// - neither `Clone` nor `Copy`. The only way to access is via `&Self` or `&mut Self` returned
///   from a [`crate::Accounts`] dispensed [`crate::AccountHandle`]
/// - the `'account` lifetime is pretty much synonymous with `'static` since the buffer it points to is valid for the entire
///   program's execution
#[derive(Debug)]
#[repr(transparent)]
pub struct Account<'account> {
    ptr: NonNull<AccountRaw>,

    // Need this to remove covariance of NonNull;
    // all `Accounts` must have the same 'account lifetime.
    //
    // TBH I dont fully get it either yet but this thing is like
    // an UnsafeCell so we should follow UnsafeCell's variance
    // https://doc.rust-lang.org/nomicon/subtyping.html#variance
    _phantom: PhantomData<&'account mut AccountRaw>,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AccountRaw {
    _duplicate_flag: u8,

    /// Indicates whether the transaction was signed by this account.
    is_signer: u8,

    /// Indicates whether the account is writable.
    is_writable: u8,

    /// Indicates whether this account represents a program.
    is_executable: u8,

    /// The number of bytes this account has already grown by
    /// from its original size. A negative value means the account
    /// has shrunk
    ///
    /// Capped at [`crate::MAX_PERMITTED_DATA_INCREASE`].
    ///
    /// Overflow safety: solana accounts have a max data size of 10Mib,
    /// well within i32 range in either +/- direction.
    ///
    /// These 4 bytes here used to be struct padding bytes,
    /// until anza decided to repurpose them
    /// as scratch space for recording data to support realloc in 1.10.
    /// Guaranteed to be zero at entrypoint time.
    realloc_budget_used: i32,

    /// Public key of the account.
    key: [u8; 32],

    /// Program that owns this account. Modifiable by programs.
    owner: [u8; 32],

    /// The lamports in the account. Modifiable by programs.
    lamports: u64,

    /// Length of the data. Modifiable by programs.
    data_len: u64,
}

const _CHECK_ACCOUNT_RAW_SIZE: () = assert!(size_of::<AccountRaw>() == 88);
const _CHECK_ACCOUN_RAW_ALIGN: () = assert!(align_of::<AccountRaw>() == 8);

/// Accessors
impl Account<'_> {
    #[inline(always)]
    const fn as_raw(&self) -> &AccountRaw {
        unsafe { self.ptr.as_ref() }
    }

    #[inline(always)]
    pub fn is_signer(&self) -> bool {
        self.as_raw().is_signer != 0
    }

    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.as_raw().is_writable != 0
    }

    #[inline(always)]
    pub fn is_executable(&self) -> bool {
        self.as_raw().is_executable != 0
    }

    #[inline(always)]
    pub fn lamports(&self) -> u64 {
        self.as_raw().lamports
    }

    /// Only used for CPI helpers.
    ///
    /// To read and manipulate lamports, use
    /// [`Self::lamports`] and [`Self::set_lamports`], [`Self::inc_lamports`],
    /// [`Self::dec_lamports`] instead.
    #[inline(always)]
    pub fn lamports_ref(&self) -> &u64 {
        &self.as_raw().lamports
    }

    #[inline(always)]
    pub fn data_len_u64(&self) -> u64 {
        self.as_raw().data_len
    }

    #[inline(always)]
    pub fn data_len(&self) -> usize {
        self.data_len_u64() as usize
    }

    #[inline(always)]
    pub fn key(&self) -> &[u8; 32] {
        &self.as_raw().key
    }

    #[inline(always)]
    pub fn owner(&self) -> &[u8; 32] {
        &self.as_raw().owner
    }
}

/// Mutators
impl Account<'_> {
    #[inline(always)]
    fn as_raw_mut(&mut self) -> &mut AccountRaw {
        unsafe { self.ptr.as_mut() }
    }

    #[inline(always)]
    pub fn set_lamports(&mut self, new_lamports: u64) {
        self.as_raw_mut().lamports = new_lamports;
    }

    #[inline(always)]
    pub fn inc_lamports(&mut self, inc_lamports: u64) -> Result<(), ProgramError> {
        match self.lamports().checked_add(inc_lamports) {
            Some(new_lamports) => {
                self.set_lamports(new_lamports);
                Ok(())
            }
            None => Err(ProgramError::ArithmeticOverflow),
        }
    }

    /// # Safety
    /// - increment must not result in overflow
    #[inline(always)]
    pub unsafe fn inc_lamports_unchecked(&mut self, inc_lamports: u64) {
        let new_lamports = self.lamports() + inc_lamports;
        self.set_lamports(new_lamports);
    }

    #[inline(always)]
    pub fn dec_lamports(&mut self, dec_lamports: u64) -> Result<(), ProgramError> {
        match self.lamports().checked_sub(dec_lamports) {
            Some(new_lamports) => {
                self.set_lamports(new_lamports);
                Ok(())
            }
            None => Err(ProgramError::InsufficientFunds),
        }
    }

    /// # Safety
    /// - decrement must not result in overflow
    #[inline(always)]
    pub unsafe fn dec_lamports_unchecked(&mut self, dec_lamports: u64) {
        let new_lamports = self.lamports() - dec_lamports;
        self.set_lamports(new_lamports);
    }

    #[inline(always)]
    pub fn assign_direct(&mut self, new_owner: [u8; 32]) {
        self.as_raw_mut().owner = new_owner;
    }
}

/// Account Data
impl Account<'_> {
    #[inline(always)]
    const fn data_ptr(&self) -> *mut u8 {
        unsafe { self.ptr.as_ptr().cast::<u8>().add(size_of::<AccountRaw>()) }
    }

    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data_ptr(), self.data_len()) }
    }

    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.data_ptr(), self.data_len()) }
    }

    #[inline(always)]
    pub fn realloc(&mut self, new_len: usize, zero_init: bool) -> Result<(), ProgramError> {
        // account data lengths should always be <= 10MiB < i32::MAX,
        let curr_len = self.data_len();
        let [new_len_i32, curr_len_i32] =
            [new_len, curr_len].map(|usz| usz.try_into().map_err(|_| ProgramError::InvalidRealloc));
        let new_len_i32: i32 = new_len_i32?;
        let curr_len_i32: i32 = curr_len_i32?;

        // unchecked-arith: all quantities are in [0, 10MiB],
        // these subtractions and additions should never overflow
        let budget_delta = new_len_i32 - curr_len_i32;
        let new_realloc_budget_used = self.as_raw().realloc_budget_used + budget_delta;
        if new_realloc_budget_used > MAX_PERMITTED_DATA_INCREASE as i32 {
            return Err(ProgramError::InvalidRealloc);
        }
        self.as_raw_mut().realloc_budget_used = new_realloc_budget_used;

        self.as_raw_mut().data_len = new_len as u64;
        if let Ok(growth) = usize::try_from(budget_delta) {
            if zero_init {
                // TODO: see if sol_memset syscall is necessary here,
                // or if ptr::write_bytes is optimized into that
                unsafe {
                    core::ptr::write_bytes(self.data_ptr().add(curr_len), 0, growth);
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn shrink_by(&mut self, dec_bytes: usize) -> Result<(), ProgramError> {
        match self.data_len().checked_sub(dec_bytes) {
            Some(new_len) => self.realloc(new_len, false),
            None => Err(ProgramError::ArithmeticOverflow),
        }
    }

    #[inline(always)]
    pub fn grow_by(&mut self, inc_bytes: usize, zero_init: bool) -> Result<(), ProgramError> {
        match self.data_len().checked_add(inc_bytes) {
            Some(new_len) => self.realloc(new_len, zero_init),
            None => Err(ProgramError::ArithmeticOverflow),
        }
    }
}

/// Pointer equality
impl PartialEq for Account<'_> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }
}

impl Eq for Account<'_> {}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn comptime_lifetimes_check() {
        let mut invalid_runtime_buffer = [];
        let (_, invalid_acc) =
            unsafe { Account::non_dup_from_ptr(invalid_runtime_buffer.as_mut_ptr()) };
        let mut invalid_accounts: Accounts<'_, 1> = Accounts {
            accounts: [MaybeUninit::new(invalid_acc)],
            len: 1,
        };

        let h = unsafe { invalid_accounts.handle_unchecked(0) };
        let _first_immut_borrow = invalid_accounts.get(h);
        let _second_immut_borrow = invalid_accounts.get(h);
        let _third_mut_borrow = invalid_accounts.get_mut(h);
        //let _fail_immut_borrow_while_mut_borrow = _second_immut_borrow; // uncomment to verify lifetime comptime error
        let _fourth_mut_borrow = invalid_accounts.get_mut(h);
        let _fifth_immut_borrow = invalid_accounts.get(h);
    }
}
