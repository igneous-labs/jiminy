use core::marker::PhantomData;

use crate::{
    Abr, AccountHandle, DATA_LEN_DEC, IS_EXECUTABLE_DEC, IS_SIGNER_DEC, IS_WRITABLE_DEC, KEY_DEC,
    LAMPORTS_DEC, OWNER_DEC,
};

/// Only legal way to obtain this is via [`crate::Abr::get`]
// whatever traits AccountHandle implements
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct Account<'a, 'account> {
    pub(crate) handle: AccountHandle<'account>,
    pub(crate) borrow: PhantomData<&'a Abr>,
}

/// Accessors
impl<'a> Account<'a, '_> {
    // 'a lifetime: borrow of any field of the account is valid
    // as long as Abr borrow is valid
    #[inline(always)]
    const fn get<T>(&self, dec: usize) -> &'a T {
        // safety: this is a private internal util for
        // use with well-known fields that have correct alignments
        // and offsets
        unsafe { &*self.handle.account_data.sub(dec).cast() }
    }

    #[inline(always)]
    const fn get_flag(&self, dec: usize) -> bool {
        *self.get::<u8>(dec) != 0
    }

    #[inline(always)]
    pub const fn is_signer(&self) -> bool {
        self.get_flag(IS_SIGNER_DEC)
    }

    #[inline(always)]
    pub const fn is_writable(&self) -> bool {
        self.get_flag(IS_WRITABLE_DEC)
    }

    #[inline(always)]
    pub const fn is_executable(&self) -> bool {
        self.get_flag(IS_EXECUTABLE_DEC)
    }

    /// Only used externally by CPI helpers.
    ///
    /// To read and manipulate lamports, use
    /// [`Self::lamports`] and [`Self::set_lamports`], [`Self::inc_lamports`],
    /// [`Self::dec_lamports`] instead.
    #[inline(always)]
    pub const fn lamports_ref(&self) -> &'a u64 {
        self.get(LAMPORTS_DEC)
    }

    #[inline(always)]
    pub const fn lamports(&self) -> u64 {
        *self.lamports_ref()
    }

    #[inline(always)]
    pub const fn key(&self) -> &'a [u8; 32] {
        self.get(KEY_DEC)
    }

    #[inline(always)]
    pub const fn owner(&self) -> &'a [u8; 32] {
        self.get(OWNER_DEC)
    }

    #[inline(always)]
    pub const fn data_len_u64(&self) -> u64 {
        *self.get(DATA_LEN_DEC)
    }

    #[inline(always)]
    pub const fn data_len(&self) -> usize {
        self.data_len_u64() as usize
    }

    /// Account data is always guaranteed to be 8-byte aligned
    #[inline(always)]
    pub const fn data(&self) -> &'a [u8] {
        unsafe { core::slice::from_raw_parts(self.handle.account_data, self.data_len()) }
    }
}
