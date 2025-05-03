//! This program prints a bunch of stuff

#![allow(unexpected_cfgs)]

use jiminy_entrypoint::program_error::ProgramError;
use jiminy_log::{msg, sol_log, sol_log_cus_remaining, sol_log_pubkey, sol_log_slice};

pub const MAX_ACCS: usize = 128;

type Accounts<'a> = jiminy_entrypoint::account::Accounts<'a, MAX_ACCS>;

type PubkeyStr = bs58_fixed::Bs58Str<44>;

jiminy_entrypoint::entrypoint!(process_ix, MAX_ACCS);

fn process_ix(
    accounts: &mut Accounts,
    data: &[u8],
    prog_id: &[u8; 32],
) -> Result<(), ProgramError> {
    sol_log_cus_remaining();

    // 204 CUs
    sol_log("Hello jiminy!");
    sol_log_cus_remaining();

    // Dont bother with `.collect::<String>()` here since that
    // requires owned Strings, adds a ton of overhead.
    // Functional programming in rust is just not meant to be.
    let mut accounts_str = String::new();
    let mut pks = PubkeyStr::new();
    accounts.as_slice().iter().for_each(|h| {
        pks.encode_from(accounts.get(*h).key());
        accounts_str += pks.as_str();
        accounts_str += ", ";
    });
    msg!("Accounts: {accounts_str}");
    sol_log_cus_remaining();

    sol_log_slice(data);
    sol_log_cus_remaining();

    // 203 CUs
    sol_log_pubkey(prog_id);
    sol_log_cus_remaining();

    Ok(())
}
