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

/// A sysvar that:
/// - is small enough to be read out in whole via the sysvar syscall
///   without exceeding stack size
/// - has the same in-memory representation as its serialized format
///   that is returned by the syscall, at least for the first few bytes
///   before external/suffix padding.
///
///   This means no internal padding between fields.
///
///   This means primitive fields must have the same endianness
///   in the serialized format as the solana vm (little-endian).
///
/// # Safety
/// - implementors must make sure the above requirements are met
pub unsafe trait SimpleSysvar: SysvarId + Sized {
    /// This is `size_of::<Self>()` for structs with no external/suffix padding,
    /// but may be shorter for types that do have it.
    const ACCOUNT_LEN: usize = core::mem::size_of::<Self>();

    #[inline]
    fn get() -> Result<Self, ProgramError> {
        #[cfg(target_os = "solana")]
        {
            let mut res = core::mem::MaybeUninit::<Self>::uninit();
            let syscall_res = unsafe {
                jiminy_syscall::sol_get_sysvar(
                    Self::ID.as_ptr(),
                    res.as_mut_ptr().cast(),
                    0,
                    Self::ACCOUNT_LEN as u64,
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

/// Implement pointer casting "serialization"
/// for `SimpleSysvar`s
///
/// # Safety
/// - should only be used for types that impl `SimpleSysvar`
#[macro_export]
macro_rules! impl_cast_to_account_data {
    ($t:ty) => {
        impl $t {
            #[inline]
            pub const fn as_account_data_arr(
                &self,
            ) -> &[u8; <Self as $crate::SimpleSysvar>::ACCOUNT_LEN] {
                // safety: SimpleSysvars means no internal struct padding.
                // Presence of external/suffix padding just means those bytes
                // are not included in the returned array ref.
                unsafe { &*core::ptr::from_ref(self).cast() }
            }
        }
    };
}

/// Implement pointer casting "serialization"
/// for `SimpleSysvar`s
///
/// # Safety
/// - should only be used for types that impl `SimpleSysvar` and
///   have `size_of::<Self>() == Self::ACCOUNT_LEN`
#[macro_export]
macro_rules! impl_cast_from_account_data {
    ($t:ty) => {
        impl $t {
            #[inline]
            pub const fn of_account_data(account_data: &[u8]) -> Result<&Self, ProgramError> {
                match account_data.len() {
                    <Self as $crate::SimpleSysvar>::ACCOUNT_LEN => unsafe {
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
            pub const fn of_account_data_arr(
                account_data_arr: &[u8; <Self as $crate::SimpleSysvar>::ACCOUNT_LEN],
            ) -> &Self {
                const {
                    assert!(
                        <Self as $crate::SimpleSysvar>::ACCOUNT_LEN == core::mem::size_of::<Self>()
                    );
                }

                // safety: SimpleSysvars means no internal struct padding
                unsafe { &*core::ptr::from_ref(account_data_arr).cast() }
            }
        }
    };
}
