use core::{marker::PhantomData, ops::Deref};

use crate::PdaSeed;

/// The seeds for a single PDA signer.
///
/// Just `&[PdaSeed]`, but in the layout that the solana syscalls (e.g. `sol_invoke_signed_c`) expects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PdaSigner<'signer, 'seed> {
    ptr: *const PdaSeed<'seed>,
    len: u64,
    _pda_seeds: PhantomData<&'signer [PdaSeed<'seed>]>,
}

impl<'signer, 'seed> PdaSigner<'signer, 'seed> {
    #[inline(always)]
    pub const fn new(seeds: &'signer [PdaSeed<'seed>]) -> Self {
        Self {
            ptr: seeds.as_ptr(),
            len: seeds.len() as u64,
            _pda_seeds: PhantomData,
        }
    }

    #[inline(always)]
    pub const fn as_slice(&self) -> &'signer [PdaSeed<'seed>] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len as usize) }
    }
}

impl<'signer, 'seed> From<&'signer [PdaSeed<'seed>]> for PdaSigner<'signer, 'seed> {
    #[inline(always)]
    fn from(value: &'signer [PdaSeed<'seed>]) -> Self {
        Self::new(value)
    }
}

impl<'seed> Deref for PdaSigner<'_, 'seed> {
    type Target = [PdaSeed<'seed>];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
