#![cfg(feature = "test-sbf")]

use jiminy_pda::{MAX_SEEDS, MAX_SEED_LEN};
use jiminy_test_utils::silence_mollusk_prog_logs;
use mollusk_svm::{program::keyed_account_for_system_program, result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

const PROG_NAME: &str = "pda_assign";

#[test]
fn pda_assign_basic() {
    // 2 seeds, one of len 0, other of len 32
    const S2_LEN: usize = MAX_SEED_LEN;
    const S2: [u8; S2_LEN] = {
        let mut s2 = [0u8; S2_LEN];
        let mut i = 0;
        while i < S2_LEN {
            s2[i] = i as u8;
            i += 1;
        }
        s2
    };
    const SEEDS: &[&[u8]] = &[&[], &S2];
    const SEED_IX_DATA_LEN: usize = S2_LEN + 2;
    const SEED_IX_DATA: [u8; SEED_IX_DATA_LEN] = {
        let mut data = [0u8; SEED_IX_DATA_LEN];
        data[1] = S2_LEN as u8;
        let mut i = 0;
        while i < S2_LEN {
            data[2 + i] = S2[i];
            i += 1;
        }
        data
    };

    let prog_id = Pubkey::new_unique();
    let (pda, _bump) = Pubkey::find_program_address(SEEDS, &prog_id);

    let svm = Mollusk::new(&prog_id, PROG_NAME);

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        resulting_accounts,
        ..
    } = svm.process_instruction(
        &Instruction::new_with_bytes(
            prog_id,
            &SEED_IX_DATA,
            vec![
                AccountMeta {
                    pubkey: system_program::ID,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: pda,
                    is_signer: false,
                    is_writable: true,
                },
            ],
        ),
        &[
            keyed_account_for_system_program(),
            (pda, Account::default()),
        ],
    );

    raw_result.unwrap();
    eprintln!("{compute_units_consumed} CUs");

    assert_eq!(resulting_accounts[1].1.owner, prog_id);
}

#[test]
fn pda_assign_max_seeds() {
    // (MAX_SEEDS - 1) seeds
    const SEED_IX_DATA: [u8; 30] = [1u8; 30];

    let prog_id = Pubkey::new_unique();
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
            &[1],
        ],
        &prog_id,
    );

    let svm = Mollusk::new(&prog_id, PROG_NAME);

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        resulting_accounts,
        ..
    } = svm.process_instruction(
        &Instruction::new_with_bytes(
            prog_id,
            &SEED_IX_DATA,
            vec![
                AccountMeta {
                    pubkey: system_program::ID,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: pda,
                    is_signer: false,
                    is_writable: true,
                },
            ],
        ),
        &[
            keyed_account_for_system_program(),
            (pda, Account::default()),
        ],
    );

    raw_result.unwrap();
    eprintln!("{compute_units_consumed} CUs");

    assert_eq!(resulting_accounts[1].1.owner, prog_id);
}

struct SeedsIxData(Vec<u8>);

impl<'a> FromIterator<&'a [u8]> for SeedsIxData {
    fn from_iter<T: IntoIterator<Item = &'a [u8]>>(iter: T) -> Self {
        let mut v = vec![];
        for seed in iter {
            v.push(seed.len() as u8);
            v.extend(seed);
        }
        Self(v)
    }
}

proptest! {
    #[test]
    fn pda_assign_correct(
        seeds in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..=MAX_SEED_LEN), 0..MAX_SEEDS),
        prog_id: [u8; 32]
    ) {
        let prog_id = Pubkey::new_from_array(prog_id);
        let SeedsIxData(ix_data) = seeds.iter().map(|v| v.as_slice()).collect();
        let seeds_for_solana_sdk: Vec<_> = seeds.iter().map(|v| v.as_slice()).collect();
        let (pda, _bump) = Pubkey::find_program_address(
            &seeds_for_solana_sdk,
            &prog_id,
        );

        let svm = Mollusk::new(&prog_id, PROG_NAME);
        silence_mollusk_prog_logs();

        let InstructionResult {
            raw_result,
            resulting_accounts,
            ..
        } = svm.process_instruction(
            &Instruction::new_with_bytes(
                prog_id,
                &ix_data,
                vec![
                    AccountMeta {
                        pubkey: system_program::ID,
                        is_signer: false,
                        is_writable: false,
                    },
                    AccountMeta {
                        pubkey: pda,
                        is_signer: false,
                        is_writable: true,
                    },
                ],
            ),
            &[
                keyed_account_for_system_program(),
                (pda, Account::default()),
            ],
        );

        raw_result.unwrap();

        prop_assert_eq!(resulting_accounts[1].1.owner, prog_id);
    }
}
