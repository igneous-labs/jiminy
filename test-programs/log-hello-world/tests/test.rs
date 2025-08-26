//! .so file size: 23_240

#![cfg(feature = "test-sbf")]

use jiminy_test_utils::silence_mollusk_prog_logs;
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "log_hello_world";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("Hr9wsgMm4A5A3eE7eobvSWzHBNrNixakzDfsmE4cQKqq");

const TEST_ACC_PK_1: Pubkey =
    solana_pubkey::pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");
const TEST_ACC_PK_2: Pubkey =
    solana_pubkey::pubkey!("FpaavSQvEQhPDoQoLUHhmBsKZsG2WJQXj7FBCSPE1TZ1");

thread_local! {
    static SVM: Mollusk = Mollusk::new(&PROG_ID, PROG_NAME);
}

// dont use msg!() in your programs, boys and girls
/// CUs: 4432
#[test]
fn log_hello_world_basic_cus() {
    const ACCS: [Pubkey; 2] = [TEST_ACC_PK_1, TEST_ACC_PK_2];

    let ixd = [1, 2, 3, 4];
    let metas = ACCS.map(|pubkey| AccountMeta {
        pubkey,
        is_signer: false,
        is_writable: false,
    });
    let accounts = ACCS.map(|pubkey| (pubkey, Account::default()));

    SVM.with(|svm| {
        let InstructionResult {
            compute_units_consumed,
            raw_result,
            ..
        } = svm.process_instruction(
            &Instruction::new_with_bytes(PROG_ID, &ixd, metas.to_vec()),
            &accounts,
        );

        raw_result.unwrap();

        eprintln!("{compute_units_consumed} CUs");
    });
}

proptest! {
    #[test]
    fn log_hello_world_any(
        accs in proptest::collection::vec(any::<[u8; 32]>(), 0..8),
        data in proptest::collection::vec(any::<u8>(), 0..128),
    ) {
        silence_mollusk_prog_logs();

        let metas: Vec<_> = accs.iter().map(|pubkey| AccountMeta {
            pubkey: Pubkey::new_from_array(*pubkey),
            is_signer: false,
            is_writable: false,
        }).collect();
        let accs: Vec<_> = accs.into_iter().map(
            |pubkey| (Pubkey::new_from_array(pubkey), Account::default())
        ).collect();

        SVM.with(|svm| {
            let InstructionResult { raw_result, .. } = svm.process_instruction(
                &Instruction::new_with_bytes(PROG_ID, &data, metas),
                &accs,
            );

            raw_result.unwrap();
        });
    }
}
