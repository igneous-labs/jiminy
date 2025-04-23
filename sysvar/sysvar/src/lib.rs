#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

// Re-exports
pub mod program_error {
    pub use jiminy_program_error::*;
}
use program_error::*;

pub trait SysvarId {
    const ID: [u8; 32];
}

/// A sysvar that is:
/// - small enough to be read out in whole via the sysvar syscall
///   without exceeding stack size
/// - has the same in-memory representation as its serialized format
///   that is returned by the syscall.
///   This means no struct padding allowed & align == 1.
pub trait SimpleSysvar: SysvarId + Copy {
    #[inline]
    fn get() -> Result<Self, ProgramError> {
        const {
            assert!(core::mem::align_of::<Self>() == 1);
        }

        #[cfg(target_os = "solana")]
        {
            let mut res = core::mem::MaybeUninit::<Self>::uninit();
            let syscall_res = unsafe {
                jiminy_syscall::sol_get_sysvar(
                    Self::ID.as_ptr(),
                    res.as_mut_ptr().cast(),
                    0,
                    core::mem::size_of::<Self>() as u64,
                )
            };
            match core::num::NonZeroU64::new(syscall_res) {
                None => Ok(unsafe { res.assume_init() }),
                Some(err) => Err(ProgramError(err)),
            }
        }

        #[cfg(not(target_os = "solana"))]
        {
            unreachable!()
        }
    }
}

/// implement [`SimpleSysvar::get()`] as an inherent method
/// so that is is available to call even if the trait is not in scope
#[macro_export]
macro_rules! inherent_simple_sysvar_get {
    () => {
        #[inline]
        pub fn sysvar_get() -> Result<Self, $crate::program_error::ProgramError> {
            <Self as $crate::SimpleSysvar>::get()
        }
    };
}
