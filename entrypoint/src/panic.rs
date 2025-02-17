use core::panic::PanicInfo;

#[macro_export]
macro_rules! default_panic_handler {
    () => {
        /// Default panic handler.
        #[cfg(target_os = "solana")]
        #[no_mangle]
        fn custom_panic(info: &core::panic::PanicInfo<'_>) {
            $crate::panic::log_panic(info);
        }
    };
}

#[inline]
pub fn log_panic(info: &PanicInfo<'_>) -> ! {
    #[cfg(target_os = "solana")]
    {
        if let Some(location) = info.location() {
            let f = location.file();
            unsafe {
                jiminy_syscall::sol_log_(f.as_ptr(), f.len() as u64);
            }
        }
        const MSG: &str = "** PANICKED **";
        unsafe { jiminy_syscall::sol_log_(MSG.as_ptr(), MSG.len() as u64) };
        loop {}
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(info);
        unreachable!()
    }
}
