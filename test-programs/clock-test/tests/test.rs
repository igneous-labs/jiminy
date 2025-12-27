#![cfg(feature = "test-sbf")]

use std::cell::RefCell;

use jiminy_test_utils::{bench_binsize, expect_test::expect, silence_mollusk_prog_logs};
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_clock::Clock as SolanaClock;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "clock_test";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("DfbFRtuFbUaYfomYMhc8EPBYrC2zopTQcYK2cuNcPCwU");

thread_local! {
    static SVM: RefCell<Mollusk> = RefCell::new(Mollusk::new(&PROG_ID, PROG_NAME));
}

fn instr() -> Instruction {
    Instruction::new_with_bytes(PROG_ID, &[], vec![])
}

#[test]
fn binsize_bench() {
    bench_binsize(PROG_NAME, expect!["1600"]);
}

#[test]
fn clock_test_basic_cus() {
    let ix = instr();
    let (
        InstructionResult {
            raw_result,
            compute_units_consumed,
            return_data,
            ..
        },
        clock,
    ) = SVM.with(|svm| {
        let svm = svm.borrow();
        (svm.process_instruction(&ix, &[]), svm.sysvars.clock.clone())
    });
    raw_result.unwrap();
    assert_eq!(bincode::serialize(&clock).unwrap(), return_data);

    expect!["224"].assert_eq(&compute_units_consumed.to_string());
}

proptest! {
    #[test]
    fn clock_matches_set(
        slot: u64,
        epoch_start_timestamp: i64,
        epoch: u64,
        leader_schedule_epoch: u64,
        unix_timestamp: i64,
    ) {
        silence_mollusk_prog_logs();

        let ix = instr();
        let (
            InstructionResult {
                raw_result,
                return_data,
                ..
            },
            clock
        ) = SVM.with(|svm| {
            let mut svm = svm.borrow_mut();
            svm.sysvars.clock = SolanaClock {
                slot,
                epoch_start_timestamp,
                epoch,
                leader_schedule_epoch,
                unix_timestamp,
            };
            (svm.process_instruction(&ix, &[]), svm.sysvars.clock.clone())
        });
        raw_result.unwrap();
        prop_assert_eq!(bincode::serialize(&clock).unwrap(), return_data);
    }
}
