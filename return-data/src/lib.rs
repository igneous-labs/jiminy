#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

use core::mem::MaybeUninit;

/// Maximum size that can be set using [`set_return_data`].
pub const MAX_RETURN_DATA: usize = 1024;

type ReturnDataLen = u16;

const _ASSERT_RETURN_DATA_LEN_BITWIDTH_SUFFICIENT: () =
    if (ReturnDataLen::MAX as usize) < MAX_RETURN_DATA {
        panic!("ReturnDataLen type bitwidth insufficient for MAX_RETURN_DATA");
    };

/// `N` must be <= [`crate::MAX_RETURN_DATA`]
#[derive(Debug, Clone, Copy)]
pub struct ReturnData<const MAX_DATA_LEN: usize = MAX_RETURN_DATA> {
    program_id: MaybeUninit<[u8; 32]>,
    len: ReturnDataLen,
    buf: [MaybeUninit<u8>; MAX_DATA_LEN],
}

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
    pub const fn data_len_raw(&self) -> ReturnDataLen {
        self.len
    }

    #[inline]
    pub const fn data_len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data_len_raw() == 0
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

/// `N` must be <= [`crate::MAX_RETURN_DATA`]
#[inline]
pub fn get_return_data<const MAX_DATA_LEN: usize>() -> Option<ReturnData<MAX_DATA_LEN>> {
    #[cfg(target_os = "solana")]
    {
        const UNINIT: MaybeUninit<u8> = MaybeUninit::uninit();
        let mut res: ReturnData<MAX_DATA_LEN> = ReturnData {
            program_id: MaybeUninit::uninit(),
            len: 0,
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
            let size = core::cmp::min(size as usize, MAX_DATA_LEN);
            // as-safety: MAX_DATA_LEN <= MAX_RETURN_DATA precondition
            res.len = size as ReturnDataLen;
            Some(res)
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        unreachable!()
    }
}
