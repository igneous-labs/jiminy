use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use crate::PdaSeed;

// rust syntax doesnt allow `const MAX_SEEDS: usize = crate::MAX_SEEDS`,
// so redeclare a const here
const M: usize = crate::MAX_SEEDS;

#[derive(Debug, Clone, Copy)]
pub struct PdaSeedArr<'seed, const MAX_SEEDS: usize = M> {
    seeds: [MaybeUninit<PdaSeed<'seed>>; MAX_SEEDS],
    len: u8, // PDAs can only have max 16 seeds
}

impl<'seed, const MAX_SEEDS: usize> PdaSeedArr<'seed, MAX_SEEDS> {
    #[inline]
    pub const fn new() -> Self {
        const UNINIT: MaybeUninit<PdaSeed<'_>> = MaybeUninit::uninit();

        Self {
            seeds: [UNINIT; MAX_SEEDS],
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, seed: PdaSeed<'seed>) -> Result<(), PdaSeed> {
        if self.is_full() {
            Err(seed)
        } else {
            unsafe {
                self.push_unchecked(seed);
            }
            Ok(())
        }
    }

    /// # Safety
    /// - self must not be full
    #[inline]
    pub unsafe fn push_unchecked(&mut self, seed: PdaSeed<'seed>) {
        self.seeds[self.len()].write(seed);
        self.len += 1;
    }

    #[inline]
    pub const fn len_u8(&self) -> u8 {
        self.len
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len_u8() as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        self.len() == MAX_SEEDS
    }

    #[inline]
    pub const fn as_slice(&self) -> &[PdaSeed<'seed>] {
        unsafe { core::slice::from_raw_parts(self.seeds.as_ptr().cast(), self.len()) }
    }

    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [PdaSeed<'seed>] {
        unsafe { core::slice::from_raw_parts_mut(self.seeds.as_mut_ptr().cast(), self.len()) }
    }
}

impl<'seed, const MAX_SEEDS: usize> FromIterator<PdaSeed<'seed>> for PdaSeedArr<'seed, MAX_SEEDS> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = PdaSeed<'seed>>>(iter: T) -> Self {
        iter.into_iter().fold(Self::new(), |mut res, seed| {
            let _maybe_discarded: Result<(), PdaSeed<'_>> = res.push(seed);
            res
        })
    }
}

impl<'seed, const MAX_SEEDS: usize> Deref for PdaSeedArr<'seed, MAX_SEEDS> {
    type Target = [PdaSeed<'seed>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const MAX_SEEDS: usize> DerefMut for PdaSeedArr<'_, MAX_SEEDS> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<const MAX_SEEDS: usize> Default for PdaSeedArr<'_, MAX_SEEDS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
