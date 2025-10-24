use crate::{
    AccountHandle, DATA_LEN_DEC, IS_EXECUTABLE_DEC, IS_SIGNER_DEC, IS_WRITABLE_DEC, KEY_DEC,
    LAMPORTS_DEC, OWNER_DEC,
};

/// Only legal way to obtain this is via [`crate::Abr::get_unsafe`].
///
/// Provides raw pointer access to the underlying account to avoid
/// UB related to creating intermediate references
///
/// Currently only used in CPI
// whatever traits AccountHandle implements
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct UnsafeAccount<'account> {
    pub(crate) handle: AccountHandle<'account>,
}

impl UnsafeAccount<'_> {
    #[inline(always)]
    const fn get<T>(self, dec: usize) -> *mut T {
        // safety: this is a private internal util for
        // use with well-known fields that have correct alignments
        // and offsets
        unsafe { self.handle.account_data.sub(dec).cast() }
    }

    #[inline(always)]
    const fn get_flag(self, dec: usize) -> bool {
        // safety: this is a private internal util for
        // use with well-known fields that have correct alignments
        // and offsets
        unsafe { *self.get::<u8>(dec) != 0 }
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn key_ptr(self) -> *mut [u8; 32] {
        self.get(KEY_DEC)
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn lamports_ptr(self) -> *mut u64 {
        self.get(LAMPORTS_DEC)
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn data_ptr(self) -> *mut u8 {
        self.handle.account_data
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn data_len(self) -> u64 {
        *self.get(DATA_LEN_DEC)
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn owner_ptr(self) -> *mut [u8; 32] {
        self.get(OWNER_DEC)
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn is_signer(self) -> bool {
        self.get_flag(IS_SIGNER_DEC)
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn is_writable(self) -> bool {
        self.get_flag(IS_WRITABLE_DEC)
    }

    /// # Safety
    /// `this` must be a valid `Account`
    #[inline(always)]
    pub const unsafe fn is_executable(self) -> bool {
        self.get_flag(IS_EXECUTABLE_DEC)
    }
}
