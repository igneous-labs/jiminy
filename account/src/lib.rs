#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

// Re-exports
pub mod program_error {
    pub use jiminy_program_error::*;
}

mod account_types;
mod consts;
mod deser;
mod handle;

pub use account_types::*;
pub use consts::*;
pub use deser::*;
pub use handle::*;

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn comptime_lifetimes_check() {
        let mut invalid_runtime_buffer = [0; 8];
        let (_, invalid_acc) =
            unsafe { AccountHandle::non_dup_from_ptr(invalid_runtime_buffer.as_mut_ptr(), &[]) };
        let invalid_accounts: DeserAccounts<'_, 1> = DeserAccounts(Accounts {
            accounts: [MaybeUninit::new(invalid_acc)],
            len: 1,
        });
        let (mut abr, handles) = invalid_accounts.etp_start();

        let h = handles.as_slice()[0];
        let _first_immut_borrow = abr.get(h);
        let _second_immut_borrow = abr.get(h);
        let _third_mut_borrow = abr.get_mut(h);
        //let _fail_immut_borrow_while_mut_borrow = _second_immut_borrow; // uncomment to verify lifetime comptime error
        let _fourth_mut_borrow = abr.get_mut(h);
        let _fifth_immut_borrow = abr.get(h);
    }
}
