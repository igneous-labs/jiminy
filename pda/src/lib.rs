#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

mod seed;
mod seed_arr;
mod signer;

pub use seed::*;
pub use seed_arr::*;
pub use signer::*;

/// Maximum length of derived `Pubkey` seed
pub const MAX_SEED_LEN: usize = 32;

/// Maximum number of seeds, **INCLUDING** the bump seed,
/// so max (MAX_SEEDS - 1) non-bump seeds.
pub const MAX_SEEDS: usize = 16;

// DO NOT #[inline(always)] the 2 PDA functions below.
// #[inline] results in lower CUs and binary sizes

/// Returns `None` in the statistically unlikely event that
/// no valid bump seeds were found
#[inline]
pub fn try_find_program_address(
    seeds: &[PdaSeed],
    program_id: &[u8; 32],
) -> Option<([u8; 32], u8)> {
    #[cfg(target_os = "solana")]
    {
        use core::mem::MaybeUninit;

        // TODO: investigate perf of zero-initialized vs MaybeUninit
        let mut pda: MaybeUninit<[u8; 32]> = MaybeUninit::uninit();
        let mut bump: MaybeUninit<u8> = MaybeUninit::uninit();
        let result = unsafe {
            jiminy_syscall::sol_try_find_program_address(
                seeds.as_ptr().cast(),
                seeds.len() as u64,
                program_id.as_ptr(),
                pda.as_mut_ptr().cast(),
                bump.as_mut_ptr().cast(),
            )
        };
        match result {
            0 => Some(unsafe { (pda.assume_init(), bump.assume_init()) }),
            _ => None,
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box((seeds, program_id));
        unreachable!()
    }
}

/// Returns `None` if provided seeds do not result in a valid PDA
#[inline]
pub fn create_program_address(seeds: &[PdaSeed], program_id: &[u8; 32]) -> Option<[u8; 32]> {
    #[cfg(target_os = "solana")]
    {
        use core::mem::MaybeUninit;

        // TODO: investigate perf of zero-initialized vs MaybeUninit
        let mut pda: MaybeUninit<[u8; 32]> = MaybeUninit::uninit();
        let result = unsafe {
            jiminy_syscall::sol_create_program_address(
                seeds.as_ptr().cast(),
                seeds.len() as u64,
                program_id.as_ptr(),
                pda.as_mut_ptr().cast(),
            )
        };
        match result {
            0 => Some(unsafe { pda.assume_init() }),
            _ => None,
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box((seeds, program_id));
        unreachable!()
    }
}
