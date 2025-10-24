#![cfg(feature = "test-sbf")]

use jiminy_test_utils::{save_binsize_to_file, save_cus_to_file, silence_mollusk_prog_logs};
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

#[test]
fn save_binsize() {
    save_binsize_to_file(PROG_NAME);
}

// dont use msg!() in your programs, boys and girls
#[test]
fn log_hello_world_basic_cus() {
    const ACCS: [Pubkey; 2] = [TEST_ACC_PK_1, TEST_ACC_PK_2];

    let ixd = [1, 2, 3, 4];
    let metas = ACCS.map(|pubkey| AccountMeta {
        pubkey,
        is_signer: false,
        is_writable: false,
    });
    let ix = Instruction::new_with_bytes(PROG_ID, &ixd, metas.to_vec());
    let accounts = ACCS.map(|pubkey| (pubkey, Account::default()));

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        ..
    } = SVM.with(|svm| svm.process_instruction(&ix, &accounts));

    raw_result.unwrap();
    save_cus_to_file("basic", compute_units_consumed);
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
        let ix = Instruction::new_with_bytes(PROG_ID, &data, metas);
        let accs: Vec<_> = accs.into_iter().map(
            |pubkey| (Pubkey::new_from_array(pubkey), Account::default())
        ).collect();

        let InstructionResult {
            raw_result,
            ..
        } = SVM.with(|svm| svm.process_instruction(
            &ix,
            &accs,
        ));
        raw_result.unwrap()
    }
}
