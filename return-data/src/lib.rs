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
        let mut res = MaybeUninit::uninit();
        Self::overwrite(&mut res)?;
        Some(unsafe { res.assume_init() })
    }

    /// Potentially more compute-efficient version [`Self::get`] by using out-pointers.
    /// Overwrites the old data in `this`.
    ///
    /// The compiler has proven to be unable to optimize away the move/copy in
    /// `MaybeUninit::assume_init()` in many cases, especially when the returned `Self` is
    /// only dropped at entrypoint exit.
    ///
    /// A memory leak can potentially occur if the initialized value in the MaybeUninits
    /// are not dropped, but this struct is Copy so its fine
    #[inline]
    pub fn overwrite(this: &mut MaybeUninit<Self>) -> Option<&mut Self> {
        const {
            assert!(MAX_DATA_LEN <= MAX_RETURN_DATA);
        }

        #[cfg(target_os = "solana")]
        {
            use core::ptr::addr_of_mut;

            let this_ptr = this.as_mut_ptr();
            let size = unsafe {
                jiminy_syscall::sol_get_return_data(
                    addr_of_mut!((*this_ptr).buf).cast(),
                    MAX_DATA_LEN as u64,
                    addr_of_mut!((*this_ptr).program_id).cast(),
                )
            };
            if size == 0 {
                None
            } else {
                // just being defensive here in case syscall returns some
                // giant size
                let size = core::cmp::min(size as usize, MAX_DATA_LEN);
                unsafe {
                    addr_of_mut!((*this_ptr).len).write(size);
                    Some(this.assume_init_mut())
                }
            }
        }

        #[cfg(not(target_os = "solana"))]
        {
            core::hint::black_box(this);
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
