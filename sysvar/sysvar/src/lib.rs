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

/// Implement pointer casting "deserialization"
/// for simple sysvars.
///
/// # Safety
/// - Can only be used with `#[repr(C, align(1))]` structs
#[macro_export]
macro_rules! impl_account_data_cast {
    ($t:ty) => {
        impl $t {
            const ACCOUNT_LEN: usize = core::mem::size_of::<Self>();
            const ACCOUNT_ALIGN: usize = core::mem::align_of::<Self>();

            #[inline]
            pub const fn of_account_data(account_data: &[u8]) -> Result<&Self, ProgramError> {
                match account_data.len() {
                    Self::ACCOUNT_LEN => unsafe {
                        Ok(Self::of_account_data_unchecked(account_data))
                    },
                    _ => Err(ProgramError::from_builtin(
                        BuiltInProgramError::InvalidAccountData,
                    )),
                }
            }

            /// # Safety
            /// - account_data must be of `size_of::<Self>()`
            #[inline]
            pub const unsafe fn of_account_data_unchecked(account_data: &[u8]) -> &Self {
                Self::of_account_data_arr(&*account_data.as_ptr().cast())
            }

            #[inline]
            pub const fn of_account_data_arr(account_data_arr: &[u8; Self::ACCOUNT_LEN]) -> &Self {
                const {
                    assert!(Self::ACCOUNT_ALIGN == 1);
                }

                // safety: align-1 checked above
                unsafe { &*core::ptr::from_ref(account_data_arr).cast() }
            }

            #[inline]
            pub const fn as_account_data_arr(&self) -> &[u8; Self::ACCOUNT_LEN] {
                const {
                    assert!(Self::ACCOUNT_ALIGN == 1);
                }

                // safety: align-1 checked above
                unsafe { &*core::ptr::from_ref(self).cast() }
            }
        }
    };
}
