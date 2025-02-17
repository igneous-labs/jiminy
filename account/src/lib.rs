#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

use core::cell::UnsafeCell;

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

/// # Implementation details
///
/// - the slice the inner ref points to is the entire data slice exclusively owned by this account, which is
///   the 88-byte account header + data_len bytes + 10kb spare space for reallocs. Excludes subsequent alignment padding bytes.
/// - inner field is an `UnsafeCell` because runtime account duplication marker mean multiple [`Account`]s may point
///   to and mutate the same data, so we need to opt out of immutability guarantee of `&`
/// - neither `Clone` nor `Copy`. The only way to access is via `&Self` or `&mut Self` returned
///   from a [`crate::Accounts`] dispensed [`crate::AccountHandle`]
/// - the `'account` lifetime is pretty much synonymous with `'static` since the buffer it points to is valid for the entire
///   program's execution
#[derive(Debug)]
#[repr(transparent)]
pub struct Account<'account>(&'account UnsafeCell<[u8]>);

/// struct offsets
impl Account<'_> {
    pub const IS_SIGNER_OFFSET: usize = 1;
    pub const IS_WRITABLE_OFFSET: usize = Self::IS_SIGNER_OFFSET + 1;
    pub const IS_EXECUTABLE_OFFSET: usize = Self::IS_WRITABLE_OFFSET + 1;

    pub const KEY_OFFSET: usize = Self::IS_EXECUTABLE_OFFSET + 5;
    pub const OWNER_OFFSET: usize = Self::KEY_OFFSET + 32;
    pub const LAMPORTS_OFFSET: usize = Self::OWNER_OFFSET + 32;
    pub const DATA_LEN_OFFSET: usize = Self::LAMPORTS_OFFSET + 8;

    pub const HEADER_LEN: usize = Self::DATA_LEN_OFFSET + 8;
    pub const DATA_OFFSET: usize = Self::HEADER_LEN;
}

/// Accessors
impl Account<'_> {
    #[inline]
    fn get_bool(&self, offset: usize) -> bool {
        let a = unsafe { &*self.0.get() };
        let byte = unsafe { a.get_unchecked(offset) };
        *byte != 0
    }

    #[inline]
    pub fn is_signer(&self) -> bool {
        self.get_bool(Self::IS_SIGNER_OFFSET)
    }

    #[inline]
    pub fn is_writable(&self) -> bool {
        self.get_bool(Self::IS_WRITABLE_OFFSET)
    }

    #[inline]
    pub fn is_executable(&self) -> bool {
        self.get_bool(Self::IS_EXECUTABLE_OFFSET)
    }

    #[inline]
    fn get_byte_slice<const N: usize>(&self, offset: usize) -> &[u8; N] {
        unsafe { &*self.0.get().cast::<u8>().add(offset).cast() }
    }

    #[inline]
    fn get_u64(&self, offset: usize) -> u64 {
        let data_len_slice: &[u8; 8] = self.get_byte_slice(offset);
        u64::from_le_bytes(*data_len_slice)
    }

    #[inline]
    pub fn lamports(&self) -> u64 {
        self.get_u64(Self::LAMPORTS_OFFSET)
    }

    /// Only used for CPI helpers.
    ///
    /// To read and manipulate lamports, use
    /// [`Self::lamports`] and [`Self::set_lamports`], [`Self::inc_lamports`],
    /// [`Self::dec_lamports`] instead.
    #[inline]
    pub fn lamports_ref(&self) -> &u64 {
        unsafe { &*self.0.get().cast::<u8>().add(Self::LAMPORTS_OFFSET).cast() }
    }

    #[inline]
    pub fn data_len_u64(&self) -> u64 {
        self.get_u64(Self::DATA_LEN_OFFSET)
    }

    #[inline]
    pub fn data_len(&self) -> usize {
        self.data_len_u64() as usize
    }

    #[inline]
    pub fn key(&self) -> &[u8; 32] {
        self.get_byte_slice(Self::KEY_OFFSET)
    }

    #[inline]
    pub fn owner(&self) -> &[u8; 32] {
        self.get_byte_slice(Self::OWNER_OFFSET)
    }
}

/// Mutators
impl Account<'_> {
    #[inline]
    fn get_byte_slice_mut<const N: usize>(&mut self, offset: usize) -> &mut [u8; N] {
        unsafe { &mut *self.0.get().cast::<u8>().add(offset).cast() }
    }

    #[inline]
    pub fn set_lamports(&mut self, new_lamports: u64) {
        *self.get_byte_slice_mut(Self::LAMPORTS_OFFSET) = new_lamports.to_le_bytes();
    }

    #[inline]
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
    #[inline]
    pub unsafe fn inc_lamports_unchecked(&mut self, inc_lamports: u64) {
        let new_lamports = self.lamports() + inc_lamports;
        self.set_lamports(new_lamports);
    }

    #[inline]
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
    #[inline]
    pub unsafe fn dec_lamports_unchecked(&mut self, dec_lamports: u64) {
        let new_lamports = self.lamports() - dec_lamports;
        self.set_lamports(new_lamports);
    }

    #[inline]
    pub fn assign_direct(&mut self, new_owner: [u8; 32]) {
        *self.get_byte_slice_mut(Self::OWNER_OFFSET) = new_owner;
    }
}

/// Account Data
impl Account<'_> {
    #[inline]
    const fn data_ptr(&self) -> *mut u8 {
        unsafe { self.0.get().cast::<u8>().add(Self::DATA_OFFSET) }
    }

    /// Returns the maximum data length this account can be reallocated to
    #[inline]
    pub fn max_data_len(&self) -> usize {
        let a = unsafe { &*self.0.get() };
        // unchecked arithmetic: len should always >= DATA_OFFSET
        a.len() - Self::DATA_OFFSET
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data_ptr(), self.data_len()) }
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.data_ptr(), self.data_len()) }
    }

    #[inline]
    pub fn realloc(&mut self, new_len: usize, zero_init: bool) -> Result<(), ProgramError> {
        if new_len > self.max_data_len() {
            return Err(ProgramError::InvalidRealloc);
        }

        let old_len = self.data_len();
        unsafe {
            self.realloc_unchecked(new_len);
        }
        if zero_init && new_len > old_len {
            // TODO: see if sol_memset syscall is necessary here,
            // or if ptr::write_bytes is optimized into that
            unsafe {
                core::ptr::write_bytes(self.data_ptr().add(old_len), 0, new_len - old_len);
            }
        }
        Ok(())
    }

    /// # Safety
    /// - new_len must be <= account's original len + [`crate::MAX_PERMITTED_DATA_INCREASE`]
    /// - this method does not zero init the new memory if size grew
    #[inline]
    pub unsafe fn realloc_unchecked(&mut self, new_len: usize) {
        *self.get_byte_slice_mut(Self::DATA_LEN_OFFSET) = (new_len as u64).to_le_bytes();
    }

    /// # Safety
    /// - dec must be <= account's original len
    #[inline]
    pub unsafe fn shrink_by_unchecked(&mut self, dec_bytes: usize) {
        let new_len = self.data_len() - dec_bytes;
        self.realloc_unchecked(new_len);
    }

    #[inline]
    pub fn shrink_by(&mut self, dec_bytes: usize) -> Result<(), ProgramError> {
        match self.data_len().checked_sub(dec_bytes) {
            Some(new_len) => {
                unsafe {
                    self.realloc_unchecked(new_len);
                }
                Ok(())
            }
            None => Err(ProgramError::ArithmeticOverflow),
        }
    }

    /// # Safety
    /// - rules of [`Self::realloc_unchecked`] apply here
    #[inline]
    pub unsafe fn grow_by_unchecked(&mut self, inc_bytes: usize) {
        let new_len = self.data_len() + inc_bytes;
        self.realloc_unchecked(new_len);
    }

    #[inline]
    pub fn grow_by(&mut self, inc_bytes: usize, zero_init: bool) -> Result<(), ProgramError> {
        match self.data_len().checked_add(inc_bytes) {
            Some(new_len) => self.realloc(new_len, zero_init),
            None => Err(ProgramError::ArithmeticOverflow),
        }
    }
}

/// Pointer equality
impl PartialEq for Account<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0.get(), other.0.get())
    }
}

impl Eq for Account<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comptime_lifetimes_check() {
        let mut invalid_runtime_buffer = [];
        let (_, invalid_acc) =
            unsafe { Account::non_dup_from_ptr(invalid_runtime_buffer.as_mut_ptr()) };
        let mut invalid_accounts: Accounts<'_, 1> = Accounts::new();
        unsafe {
            invalid_accounts.push_unchecked(invalid_acc);
        }

        let h = unsafe { invalid_accounts.handle_unchecked(0) };
        let _first_immut_borrow = invalid_accounts.get(h);
        let _second_immut_borrow = invalid_accounts.get(h);
        let _third_mut_borrow = invalid_accounts.get_mut(h);
        //let _fail_immut_borrow_while_mut_borrow = _second_immut_borrow.0; // uncomment to verify lifetime comptime error
        let _fourth_mut_borrow = invalid_accounts.get_mut(h);
        let _fifth_immut_borrow = invalid_accounts.get(h);
    }
}
