//! .so file size: 1328

#![cfg(feature = "test-sbf")]

use mollusk_svm::{result::InstructionResult, Mollusk};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "noop";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("7sw5pYQWyFKuVcztPVcMYomsHMVYoRp24rcaKxvjwnex");

/// CUs: 2
#[test]
fn noop_empty_ix_cus() {
    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let InstructionResult {
        raw_result,
        compute_units_consumed,
        ..
    } = svm.process_instruction(&Instruction::new_with_bytes(PROG_ID, &[], Vec::new()), &[]);

    raw_result.unwrap();

    eprintln!("{compute_units_consumed} CUs");
}
