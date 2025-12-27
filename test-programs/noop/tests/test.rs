#![cfg(feature = "test-sbf")]

use jiminy_test_utils::{bench_binsize, expect_test::expect, save_cus_to_file};
use mollusk_svm::{result::InstructionResult, Mollusk};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "noop";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("7sw5pYQWyFKuVcztPVcMYomsHMVYoRp24rcaKxvjwnex");

#[test]
fn binsize_bench() {
    bench_binsize(PROG_NAME, expect!["1336"]);
}

#[test]
fn noop_empty_ix_cus() {
    let svm = Mollusk::new(&PROG_ID, PROG_NAME);
    let InstructionResult {
        raw_result,
        compute_units_consumed,
        ..
    } = svm.process_instruction(&Instruction::new_with_bytes(PROG_ID, &[], Vec::new()), &[]);
    raw_result.unwrap();
    save_cus_to_file("basic", compute_units_consumed);
}
