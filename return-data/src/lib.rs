#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

use core::mem::MaybeUninit;

/// Maximum size that can be set using [`set_return_data`].
pub const MAX_RETURN_DATA: usize = 1024;

/// `MAX_DATA_LEN` must be <= [`crate::MAX_RETURN_DATA`]
#[derive(Debug, Clone, Copy)]
pub struct ReturnData<const MAX_DATA_LEN: usize = MAX_RETURN_DATA> {
    // 1024 fits into a u16 but just using usize here
    // to avoid potential non ebpf ALU supported operations
    len: usize,

    program_id: MaybeUninit<[u8; 32]>,
    buf: [MaybeUninit<u8>; MAX_DATA_LEN],
}

// Accessors
impl<const MAX_DATA_LEN: usize> ReturnData<MAX_DATA_LEN> {
    #[inline]
    pub const fn program_id(&self) -> &[u8; 32] {
        unsafe { self.program_id.assume_init_ref() }
    }

    #[inline]
    pub const fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr().cast(), self.data_len()) }
    }

    #[inline]
    pub const fn data_len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data_len() == 0
    }
}

#[inline]
pub fn set_return_data(data: &[u8]) {
    #[cfg(target_os = "solana")]
    unsafe {
        jiminy_syscall::sol_set_return_data(data.as_ptr(), data.len() as u64);
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(data);
        unreachable!()
    }
}

impl<const MAX_DATA_LEN: usize> ReturnData<MAX_DATA_LEN> {
    /// Return data copied via syscall is truncated to `MAX_DATA_LEN`.
    ///
    /// Returns `None` if syscall returned size 0 i.e. no program has set
    /// return data so far or return data has been cleared.
    #[inline]
    pub fn get() -> Option<Self> {
        #[cfg(target_os = "solana")]
        {
            const UNINIT: MaybeUninit<u8> = MaybeUninit::uninit();

            let mut res = Self {
                len: 0,
                program_id: MaybeUninit::uninit(),
                buf: [UNINIT; MAX_DATA_LEN],
            };
            let size = unsafe {
                jiminy_syscall::sol_get_return_data(
                    res.buf.as_mut_ptr().cast(),
                    MAX_DATA_LEN as u64,
                    res.program_id.as_mut_ptr().cast(),
                )
            };
            if size == 0 {
                None
            } else {
                // just being defensive here
                let size = core::cmp::min(size as usize, MAX_DATA_LEN);
                res.len = size;
                Some(res)
            }
        }

        #[cfg(not(target_os = "solana"))]
        {
            unreachable!()
        }
    }
}

/// For API symmetry with [`set_return_data`].
///
/// Uses [`ReturnData::get`] under the hood, and is probably more ergonomic
/// to just use that since the const generics can be inferred in some contexts
#[inline]
pub fn get_return_data<const MAX_DATA_LEN: usize>() -> Option<ReturnData<MAX_DATA_LEN>> {
    ReturnData::<MAX_DATA_LEN>::get()
}
