#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

mod seed;
mod seed_arr;
mod signer;

use core::mem::MaybeUninit;

pub use seed::*;
pub use seed_arr::*;
pub use signer::*;

/// Maximum length of derived `Pubkey` seed
pub const MAX_SEED_LEN: usize = 32;

/// Maximum number of seeds, **INCLUDING** the bump seed,
/// so max (MAX_SEEDS - 1) non-bump seeds.
pub const MAX_SEEDS: usize = 16;

// DO NOT #[inline(always)] the PDA functions below.
// #[inline] results in lower CUs and binary sizes

/// Returns `None` in the statistically unlikely event that
/// no valid bump seeds were found
#[inline]
pub fn try_find_program_address(
    seeds: &[PdaSeed],
    program_id: &[u8; 32],
) -> Option<([u8; 32], u8)> {
    let mut pda = MaybeUninit::uninit();
    let mut bump = MaybeUninit::uninit();
    try_find_program_address_to(seeds, program_id, &mut pda, &mut bump)?;
    Some(unsafe { (pda.assume_init(), bump.assume_init()) })
}

// need different lifetimes for mut references because of lifetime invariance

/// Returns `None` in the statistically unlikely event that
/// no valid bump seeds were found
///
/// This is potentially more compute-efficient than [`try_find_program_address`] by explicitly specifying
/// the out-pointers.
///
/// The compiler has proven to be unable to optimize away the move/copy in
/// `MaybeUninit::assume_init()` in many cases, especially when the returned `Self` is
/// only dropped at entrypoint exit.
///
/// A memory leak can potentially occur if the initialized value in the MaybeUninits
///  are not dropped, but both `pda` and `bump` are Copy so its fine
#[inline]
pub fn try_find_program_address_to<'pda, 'bump>(
    seeds: &[PdaSeed],
    program_id: &[u8; 32],
    pda_dst: &'pda mut MaybeUninit<[u8; 32]>,
    bump_dst: &'bump mut MaybeUninit<u8>,
) -> Option<(&'pda mut [u8; 32], &'bump mut u8)> {
    #[cfg(target_os = "solana")]
    {
        let result = unsafe {
            jiminy_syscall::sol_try_find_program_address(
                seeds.as_ptr().cast(),
                seeds.len() as u64,
                program_id.as_ptr(),
                pda_dst.as_mut_ptr().cast(),
                bump_dst.as_mut_ptr().cast(),
            )
        };
        match result {
            0 => Some(unsafe { (pda_dst.assume_init_mut(), bump_dst.assume_init_mut()) }),
            _ => None,
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box((seeds, program_id, pda_dst, bump_dst));
        unreachable!()
    }
}

/// Returns `None` if provided seeds do not result in a valid PDA
#[inline]
pub fn create_program_address(seeds: &[PdaSeed], program_id: &[u8; 32]) -> Option<[u8; 32]> {
    let mut pda = MaybeUninit::uninit();
    create_program_address_to(seeds, program_id, &mut pda)?;
    Some(unsafe { pda.assume_init() })
}

/// Returns `None` if provided seeds do not result in a valid PDA
///
/// This is potentially more compute-efficient than [`create_program_address`] by explicitly specifying
/// the out-pointers.
///
/// The compiler has proven to be unable to optimize away the move/copy in
/// `MaybeUninit::assume_init()` in many cases, especially when the returned `Self` is
/// only dropped at entrypoint exit.
///
/// A memory leak can potentially occur if the initialized value in the MaybeUninits
///  are not dropped, but [u8; 32] is Copy so its fine
#[inline]
pub fn create_program_address_to<'dst>(
    seeds: &[PdaSeed],
    program_id: &[u8; 32],
    pda: &'dst mut MaybeUninit<[u8; 32]>,
) -> Option<&'dst mut [u8; 32]> {
    #[cfg(target_os = "solana")]
    {
        let result = unsafe {
            jiminy_syscall::sol_create_program_address(
                seeds.as_ptr().cast(),
                seeds.len() as u64,
                program_id.as_ptr(),
                pda.as_mut_ptr().cast(),
            )
        };
        match result {
            0 => Some(unsafe { pda.assume_init_mut() }),
            _ => None,
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box((seeds, program_id, pda));
        unreachable!()
    }
}
