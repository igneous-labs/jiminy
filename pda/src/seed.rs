use core::{marker::PhantomData, ops::Deref};

/// `&[u8]`, but in the layout that sol_invoke_signed_c expects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PdaSeed<'seed> {
    ptr: *const u8,
    len: u64,
    _byte_slice: PhantomData<&'seed [u8]>,
}

impl PdaSeed<'_> {
    #[inline(always)]
    pub const fn new(seed: &[u8]) -> Self {
        Self {
            ptr: seed.as_ptr(),
            len: seed.len() as u64,
            _byte_slice: PhantomData,
        }
    }

    #[inline(always)]
    pub const fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len as usize) }
    }
}

impl From<&[u8]> for PdaSeed<'_> {
    #[inline(always)]
    fn from(value: &[u8]) -> Self {
        Self::new(value)
    }
}

impl Deref for PdaSeed<'_> {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
