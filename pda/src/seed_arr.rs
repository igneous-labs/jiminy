use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use crate::PdaSeed;

// rust syntax doesnt allow `const MAX_SEEDS: usize = crate::MAX_SEEDS`,
// so redeclare a const here
const M: usize = crate::MAX_SEEDS;

/// An owned array of [`PdaSeed`]s, representing a single [`crate::PdaSigner`]
#[derive(Debug, Clone, Copy)]
pub struct PdaSeedArr<'seed, const MAX_SEEDS: usize = M> {
    seeds: [MaybeUninit<PdaSeed<'seed>>; MAX_SEEDS],

    // PDAs can only have max M=16 seeds, but we use usize
    // here instead of u8 because ebpf only has 32-bit or 64-bit arithmetic.
    // 8-byte alignment also means we dont save any space if we use u8 anyway.
    len: usize,
}

impl<'seed, const MAX_SEEDS: usize> PdaSeedArr<'seed, MAX_SEEDS> {
    #[inline(always)]
    pub const fn new() -> Self {
        const UNINIT: MaybeUninit<PdaSeed<'_>> = MaybeUninit::uninit();

        Self {
            seeds: [UNINIT; MAX_SEEDS],
            len: 0,
        }
    }

    #[inline(always)]
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
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, seed: PdaSeed<'seed>) {
        self.seeds[self.len()].write(seed);
        self.len += 1;
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.len() == MAX_SEEDS
    }

    #[inline(always)]
    pub const fn as_slice(&self) -> &[PdaSeed<'seed>] {
        unsafe { core::slice::from_raw_parts(self.seeds.as_ptr().cast(), self.len()) }
    }

    #[inline(always)]
    pub fn as_slice_mut(&mut self) -> &mut [PdaSeed<'seed>] {
        unsafe { core::slice::from_raw_parts_mut(self.seeds.as_mut_ptr().cast(), self.len()) }
    }
}

impl<'seed, const MAX_SEEDS: usize> FromIterator<PdaSeed<'seed>> for PdaSeedArr<'seed, MAX_SEEDS> {
    /// Discards any seeds past `MAX_SEEDS`
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = PdaSeed<'seed>>>(iter: T) -> Self {
        const UNINIT: MaybeUninit<PdaSeed<'_>> = MaybeUninit::uninit();

        // probably more functional to have seeds array as part of fold accumulator
        // but i dont trust the compiler codegen after its let me down before
        let mut seeds = [UNINIT; MAX_SEEDS];
        let len = iter
            .into_iter()
            .take(MAX_SEEDS)
            .enumerate()
            .fold(0, |len, (i, seed)| {
                // index-safety: bounds checked by take(MAX_SEEDS)
                seeds[i].write(seed);
                len + 1
            });

        Self { seeds, len }
    }
}

impl<'seed, const MAX_SEEDS: usize> Deref for PdaSeedArr<'seed, MAX_SEEDS> {
    type Target = [PdaSeed<'seed>];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const MAX_SEEDS: usize> DerefMut for PdaSeedArr<'_, MAX_SEEDS> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<const MAX_SEEDS: usize> Default for PdaSeedArr<'_, MAX_SEEDS> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
