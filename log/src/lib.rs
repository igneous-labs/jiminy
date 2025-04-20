//! All functions in here simply no-ops if not called from within the VM

#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![allow(unexpected_cfgs)]

/// Print a Rust [format strings][fs] message to the log.
///
/// To print a simple string, DO NOT USE THIS MACRO.
/// Instead, simply call [`sol_log`].
///
/// The input tokens will be passed through the
/// [`format!`] macro before being logged with `sol_log`.
///
/// This macro is only available with crate `feature = "std"`
///
/// [fs]: https://doc.rust-lang.org/std/fmt/
/// [`format!`]: https://doc.rust-lang.org/std/fmt/fn.format.html
///
/// Note that Rust's formatting machinery is relatively CPU-intensive
/// for constrained environments like the Solana VM.
///
/// # Examples
///
/// ```
/// use jiminy_log::msg;
///
/// let err = "not enough signers";
/// msg!("multisig failed: {}", err);
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! msg {
    ($($arg:tt)*) => {
        $crate::sol_log(&format!($($arg)*));
    };
}

/// Logs a string to an individual line
///
/// # Example
///
/// ```
/// use jiminy_log::sol_log;
///
/// sol_log("hello world");
/// ```
///
/// will print
///
/// ```md
/// Program log: hello world
/// ```
#[inline]
pub fn sol_log(message: &str) {
    #[cfg(target_os = "solana")]
    {
        unsafe {
            jiminy_syscall::sol_log_(message.as_ptr(), message.len() as u64);
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(message);
    }
}

/// Logs the compute units remaining to an individual line
///
/// # Example
///
/// ```rust
/// use jiminy_log::sol_log_cus_remaining;
///
/// sol_log_cus_remaining();
/// ```
///
/// will print something like
///
/// ```md
/// Program consumption: 1399632 units remaining
/// ```
#[inline]
pub fn sol_log_cus_remaining() {
    #[cfg(target_os = "solana")]
    {
        unsafe {
            jiminy_syscall::sol_log_compute_units_();
        }
    }
}

/// Logs a byte slice in base64 format to an individual line
///
/// # Example
///
/// ```rust
/// use jiminy_log::sol_log_slice;
///
/// sol_log_slice(&[1, 2, 3, 4]);
/// ```
///
/// will output
///
/// ```md
/// Program data: AQIDBA==
/// ```
#[inline]
pub fn sol_log_slice(data: &[u8]) {
    #[cfg(target_os = "solana")]
    {
        #[repr(C)]
        struct ByteSlice {
            ptr: *const u8,
            len: u64,
        }

        let a = ByteSlice {
            ptr: data.as_ptr(),
            len: data.len() as u64,
        };
        unsafe {
            jiminy_syscall::sol_log_data(core::ptr::addr_of!(a).cast(), 1);
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(data);
    }
}

/// Logs a pubkey in base58 format to an individual line
///
/// # Example
///
/// Example output
///
/// ```md
/// Program log: Hr9wsgMm4A5A3eE7eobvSWzHBNrNixakzDfsmE4cQKqq
/// ```
#[inline]
pub fn sol_log_pubkey(pubkey: &[u8; 32]) {
    #[cfg(target_os = "solana")]
    {
        unsafe {
            jiminy_syscall::sol_log_pubkey(pubkey.as_ptr());
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        core::hint::black_box(pubkey);
    }
}
