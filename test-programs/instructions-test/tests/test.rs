//! .so file size:

#![cfg(feature = "test-sbf")]

use mollusk_svm::{result::InstructionResult, Mollusk};
use solana_account::Account;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "instructions_test";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("GRQbyvXVpwuQdRTHuVLzYLX7zoduu9caY73mQM8vL6jA");

fn instr() -> Instruction {
    Instruction::new_with_bytes(PROG_ID, &[], vec![])
}

/// CUs:
#[test]
fn instructions_test_basic_cus() {
    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let ix = instr();

    let InstructionResult {
        raw_result,
        compute_units_consumed,
        return_data,
        ..
    } = svm.process_instruction(
        &ix,
        &[(
            Pubkey::new_from_array(jiminy_sysvar_instructions::ID),
            Account::default(),
        )],
    );

    raw_result.unwrap();

    eprintln!("{compute_units_consumed} CUs");

    eprintln!("{return_data:#?}");
}
