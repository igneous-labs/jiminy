#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

// Re-exports
pub mod account {
    pub use jiminy_account::*;
}
pub mod program_error {
    pub use jiminy_account::program_error::*;
}

use account::*;

#[cfg(feature = "allocator")]
pub mod allocator;

#[cfg(feature = "panic")]
pub mod panic;

/// Return value for a successful program execution.
pub const SUCCESS: u64 = 0;

/// Re-export for use in exported macros
pub const MAX_TX_ACCOUNTS: usize = jiminy_account::MAX_TX_ACCOUNTS;

// use $process_instruction:expr instead of $process_instruction:ident so that
// you can use any function pointer in general not just identifiers in current scope
#[cfg(all(feature = "allocator", feature = "panic"))]
#[macro_export]
macro_rules! entrypoint {
    ( $process_instruction:expr ) => {
        $crate::entrypoint!($process_instruction, { $crate::MAX_TX_ACCOUNTS });
    };
    ( $process_instruction:expr, $maximum:expr ) => {
        $crate::program_entrypoint!($process_instruction, $maximum);
        $crate::default_allocator!();
        $crate::default_panic_handler!();
    };
}

/// Declare the program entrypoint.
///
/// This macro is similar to the `entrypoint!` macro, but it does not set up a global allocator
/// nor a panic handler. This is useful when the program will set up its own allocator and panic
/// handler.
#[macro_export]
macro_rules! program_entrypoint {
    ( $process_instruction:expr ) => {
        $crate::program_entrypoint!($process_instruction, { $crate::MAX_TX_ACCOUNTS });
    };
    ( $process_instruction:expr, $maximum:expr ) => {
        /// Program entrypoint.
        #[no_mangle]
        pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
            const _ASSERT_MAX_WITHIN_RANGE: () = if $maximum > u8::MAX as usize {
                panic!("max accounts must be <= u8::MAX")
            };

            let (accounts, instruction_data, program_id) = $crate::deserialize::<$maximum>(input);

            let (mut abr, accounts) = accounts.etp_start();

            match $process_instruction(&mut abr, accounts.as_slice(), instruction_data, program_id)
            {
                Ok(()) => $crate::SUCCESS,
                Err(error) => error.into(),
            }
        }
    };
}

/// Returned borrowed views are of data that is valid for the remainder of the program, so 'static is the
/// correct lifetime to use rather than introducing an unbounded `<'a>` lifetime.
///
/// # Safety
/// - input must be a pointer returned by the solana runtime pointing to the start of the block
///   of program input memory (0x400000000)
#[inline]
pub unsafe fn deserialize<const MAX_ACCOUNTS: usize>(
    input: *mut u8,
) -> (
    DeserAccounts<'static, MAX_ACCOUNTS>,
    &'static [u8],
    &'static [u8; 32],
) {
    let (input, accounts) = deser_accounts(&(), input);
    // cast-safety: input is 8-byte aligned after deserializing all accounts
    let ix_data_len = input.cast::<u64>().read() as usize;

    let input = input.add(8);
    let ix_data = core::slice::from_raw_parts(input, ix_data_len);

    let input = input.add(ix_data_len);
    let prog_id: &[u8; 32] = &*input.cast();

    (accounts, ix_data, prog_id)
}

#[cfg(test)]
mod tests {
    use super::{program_error::ProgramError, *};

    /// Can only have 1 unit-test like this due to no_mangle of fn entrypoint()
    #[test]
    fn comptime_check_entrypoint_types_generic() {
        fn process_ix_const_generic(
            _abr: &mut Abr,
            _accounts: &[AccountHandle<'_>],
            _data: &[u8],
            _prog_id: &[u8; 32],
        ) -> Result<(), ProgramError> {
            Ok(())
        }

        crate::entrypoint!(process_ix_const_generic, 255);
    }
}
