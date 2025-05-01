//! .so file size: 1592

#![cfg(feature = "test-sbf")]

use jiminy_test_utils::silence_mollusk_prog_logs;
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_clock::Clock as SolanaClock;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "clock_test";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("DfbFRtuFbUaYfomYMhc8EPBYrC2zopTQcYK2cuNcPCwU");

fn instr() -> Instruction {
    Instruction::new_with_bytes(PROG_ID, &[], vec![])
}

/// CUs: 224
#[test]
fn clock_test_basic_cus() {
    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let ix = instr();

    let InstructionResult {
        raw_result,
        compute_units_consumed,
        return_data,
        ..
    } = svm.process_instruction(&ix, &[]);

    raw_result.unwrap();

    eprintln!("{compute_units_consumed} CUs");

    assert_eq!(bincode::serialize(&svm.sysvars.clock).unwrap(), return_data);
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
        let mut svm = Mollusk::new(&PROG_ID, PROG_NAME);
        silence_mollusk_prog_logs();

        svm.sysvars.clock = SolanaClock {
            slot,
            epoch_start_timestamp,
            epoch,
            leader_schedule_epoch,
            unix_timestamp,
        };

        let ix = instr();

        let InstructionResult {
            raw_result,
            return_data,
            ..
        } = svm.process_instruction(&ix, &[]);

        raw_result.unwrap();

        assert_eq!(bincode::serialize(&svm.sysvars.clock).unwrap(), return_data);
    }
}
