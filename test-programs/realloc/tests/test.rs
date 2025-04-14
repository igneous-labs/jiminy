//! .so file size: 2280

#![cfg(feature = "test-sbf")]

use std::cmp::min;

use jiminy_entrypoint::account::{MAX_PERMITTED_DATA_INCREASE, MAX_PERMITTED_DATA_LENGTH};
use jiminy_test_utils::silence_mollusk_prog_logs;
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey,
    pubkey::Pubkey,
};

const PROG_NAME: &str = "realloc";
const PROG_ID: Pubkey = pubkey!("7A87rRA9qxBzRaJr7a8dHcmsPW3QfbnH63SjFzZSoz4Q");

const TEST_ACC_PK: Pubkey = pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");

fn test_realloc_acc(data_len: usize) -> Account {
    Account {
        lamports: u64::MAX,      // rent-exempt for all possible lengths
        data: vec![1; data_len], // set all bytes to 1
        owner: PROG_ID,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

fn ix_data(r1: usize, r2: usize) -> [u8; 16] {
    let mut res = [0u8; 16];
    res[..8].copy_from_slice(&r1.to_le_bytes());
    res[8..].copy_from_slice(&r2.to_le_bytes());
    res
}

fn expected_account_data(original: usize, r1: usize, r2: usize) -> Vec<u8> {
    let mut res = vec![0u8; r2];
    let end_1 = min(r1, original);
    res[0..min(end_1, r2)].fill(1);
    res
}

/// CUs: 92
#[test]
fn realloc_basic_cus() {
    let a1 = test_realloc_acc(69);
    let a1_meta = AccountMeta {
        pubkey: TEST_ACC_PK,
        is_signer: false,
        is_writable: true,
    };
    let ixd = ix_data(1, 31);
    let metas = vec![a1_meta.clone()];

    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        resulting_accounts,
        ..
    } = svm.process_instruction(
        &Instruction::new_with_bytes(PROG_ID, &ixd, metas),
        &[(TEST_ACC_PK, a1)],
    );

    raw_result.unwrap();

    eprintln!("{compute_units_consumed} CUs");

    let data = &resulting_accounts[0].1.data;
    assert_eq!(data, &expected_account_data(69, 1, 31));
}

fn valid_reallocs() -> impl Strategy<Value = (usize, usize, usize)> {
    (0..=MAX_PERMITTED_DATA_LENGTH).prop_flat_map(|original| {
        (
            Just(original),
            0..=min(
                MAX_PERMITTED_DATA_LENGTH,
                original + MAX_PERMITTED_DATA_INCREASE,
            ),
            0..=min(
                MAX_PERMITTED_DATA_LENGTH,
                original + MAX_PERMITTED_DATA_INCREASE,
            ),
        )
    })
}

proptest! {
    #[test]
    fn test_valid_reallocs((original, r1, r2) in valid_reallocs()) {
        let a1 = test_realloc_acc(original);
        let a1_meta = AccountMeta {
            pubkey: TEST_ACC_PK,
            is_signer: false,
            is_writable: true,
        };
        let ixd = ix_data(r1, r2);
        let metas = vec![a1_meta.clone()];

        let svm = Mollusk::new(&PROG_ID, PROG_NAME);
        silence_mollusk_prog_logs();

        let InstructionResult {
            raw_result,
            resulting_accounts,
            ..
        } = svm.process_instruction(
            &Instruction::new_with_bytes(PROG_ID, &ixd, metas),
            &[(TEST_ACC_PK, a1)],
        );

        raw_result.unwrap();

        let data = &resulting_accounts[0].1.data;
        prop_assert_eq!(data, &expected_account_data(original, r1, r2));
    }
}

fn invalid_reallocs() -> impl Strategy<Value = (usize, usize)> {
    (0..=MAX_PERMITTED_DATA_LENGTH)
        .prop_flat_map(|original| (Just(original), original + MAX_PERMITTED_DATA_INCREASE + 1..))
}

proptest! {
    #[test]
    fn test_invalid_reallocs((original, r1,) in invalid_reallocs()) {
        let a1 = test_realloc_acc(original);
        let a1_meta = AccountMeta {
            pubkey: TEST_ACC_PK,
            is_signer: false,
            is_writable: true,
        };
        let ixd = ix_data(r1, 0);
        let metas = vec![a1_meta.clone()];

        let svm = Mollusk::new(&PROG_ID, PROG_NAME);
        silence_mollusk_prog_logs();

        let InstructionResult { raw_result, .. } = svm.process_instruction(
            &Instruction::new_with_bytes(PROG_ID, &ixd, metas),
            &[(TEST_ACC_PK, a1)],
        );

        prop_assert_eq!(raw_result.unwrap_err(), InstructionError::InvalidRealloc);
    }
}
