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
                panic!("max accounts must be < u8::MAX")
            };

            let (mut accounts, instruction_data, program_id) =
                $crate::deserialize::<$maximum>(input);

            match $process_instruction(&mut accounts, instruction_data, program_id) {
                Ok(()) => $crate::SUCCESS,
                Err(error) => error.into(),
            }
        }
    };
}

/// # Safety
/// - input must be a pointer returned by the solana runtime pointing to the start of the block
///   of program input memory (0x400000000)
#[inline]
pub unsafe fn deserialize<'prog, const MAX_ACCOUNTS: usize>(
    input: *mut u8,
) -> (Accounts<'prog, MAX_ACCOUNTS>, &'prog [u8], &'prog [u8; 32]) {
    let total_accounts: &[u8; 8] = &*input.cast();
    let total_accounts = u64::from_le_bytes(*total_accounts) as usize;
    let input = input.add(8);

    let mut accounts_deser: SavingAccountsDeser<'prog, MAX_ACCOUNTS> =
        SavingAccountsDeser::new(input, total_accounts);
    // consume the iterator to get to max cap or end of accounts section
    accounts_deser.by_ref().count();
    let (mut discarding_accounts_deser, accounts) = accounts_deser.finish();

    // consume iterator to get to end of accounts section
    discarding_accounts_deser.by_ref().count();
    let input = discarding_accounts_deser.finish_unchecked();

    let ix_data_len_buf: &[u8; 8] = &*input.cast();
    let ix_data_len = u64::from_le_bytes(*ix_data_len_buf) as usize;

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
        fn process_ix_const_generic<const MAX_ACCOUNTS: usize>(
            _accounts: &mut Accounts<'_, MAX_ACCOUNTS>,
            _data: &[u8],
            _prog_id: &[u8; 32],
        ) -> Result<(), ProgramError> {
            Ok(())
        }

        crate::entrypoint!(process_ix_const_generic, 255);
    }
}
