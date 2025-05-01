//! ## References
//! - [serialization format of Instructions sysvar](https://github.com/anza-xyz/solana-sdk/blob/691d3064149e732f105d6ac52b80065f09041fb8/instructions-sysvar/src/lib.rs#L84-L129). Just read the code, the comments are messed up.
//!
//! TODO: current impl doesnt work, Instructions need to be passed in as Account, inaccessible via sol_get_sysvar()

#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

// Re-exports
pub mod program_error {
    pub use jiminy_sysvar::program_error::*;
}
pub mod sysvar {
    pub use jiminy_sysvar::*;
}

use core::mem::MaybeUninit;

use jiminy_sysvar::SysvarId;
use program_error::ProgramError;

pub const ID_STR: &str = "Sysvar1nstructions1111111111111111111111111";

pub const ID: [u8; 32] = const_crypto::bs58::decode_pubkey(ID_STR);

// data offsets
pub const N_IXS_OFFSET: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instructions;

impl SysvarId for Instructions {
    const ID: [u8; 32] = ID;
}

/// Number of instructions
impl Instructions {
    #[inline]
    pub fn load_n_ixs() -> Result<u16, ProgramError> {
        let mut dst = MaybeUninit::uninit();
        Self::load_n_ixs_to(&mut dst)?;
        Ok(unsafe { dst.assume_init() })
    }

    #[inline]
    pub fn load_n_ixs_to(dst: &mut MaybeUninit<u16>) -> Result<&mut u16, ProgramError> {
        write_to::<_, N_IXS_OFFSET>(dst)
    }
}

/// Bound by Copy trait to ensure no memory leak from
/// initialized value in MaybeUninit not being dropped
#[inline]
fn write_to<T: Copy, const OFFSET: usize>(
    dst: &mut MaybeUninit<T>,
) -> Result<&mut T, ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let syscall_res = unsafe {
            sysvar::sol_get_sysvar(
                ID.as_ptr(),
                dst.as_mut_ptr().cast(),
                OFFSET as u64,
                core::mem::size_of::<T>() as u64,
            )
        };
        match core::num::NonZeroU64::new(syscall_res) {
            None => Ok(unsafe { dst.assume_init_mut() }),
            Some(err) => Err(ProgramError(err)),
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(dst);
        unreachable!()
    }
}
