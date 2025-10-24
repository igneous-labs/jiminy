#![cfg(feature = "test-sbf")]

use jiminy_test_utils::{
    save_binsize_to_file, save_cus_to_file, silence_mollusk_prog_logs, two_different_pubkeys,
};
use mollusk_svm::{
    program::keyed_account_for_system_program,
    result::{Check, InstructionResult},
    Mollusk,
};
use proptest::prelude::*;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "rent_test";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("6zojiaZkiViGs8L21xXGjttFmFt2hRuzCSd9UXXnkZp4");

thread_local! {
    static SVM: Mollusk = Mollusk::new(&PROG_ID, PROG_NAME);
}

const ACC_IDX: usize = 1;

/// None means use Rent::get() sysvar to determine min balance
fn setup(
    payer: Pubkey,
    acc: Pubkey,
    size: usize,

    // TODO: this param isnt used right now, original intention
    // was to set 1 lamport less than the None case then assert failure
    // to ensure we are right at rent-exemption
    lamports: Option<u64>,
) -> (Instruction, [(Pubkey, Account); 3]) {
    let mut data = Vec::from((size as u64).to_le_bytes());
    if let Some(lamports) = lamports {
        data.extend_from_slice(&lamports.to_le_bytes());
    }
    (
        Instruction::new_with_bytes(
            PROG_ID,
            &data,
            vec![
                AccountMeta {
                    pubkey: payer,
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: acc,
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: solana_system_program::id(),
                    is_signer: false,
                    is_writable: false,
                },
            ],
        ),
        [
            (
                payer,
                Account {
                    // all the SOL he can spend
                    lamports: u64::MAX,
                    ..Default::default()
                },
            ),
            (acc, Account::default()),
            keyed_account_for_system_program(),
        ],
    )
}

#[test]
fn save_binsize() {
    save_binsize_to_file(PROG_NAME);
}

#[test]
fn rent_test_basic_cus() {
    const PAYER: Pubkey = solana_pubkey::pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");
    const ACC: Pubkey = solana_pubkey::pubkey!("7A87rRA9qxBzRaJr7a8dHcmsPW3QfbnH63SjFzZSoz4Q");
    const DATA_LEN: usize = 69;

    let (ix, accounts) = setup(PAYER, ACC, DATA_LEN, None);

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        resulting_accounts,
        ..
    } = SVM.with(|svm| {
        svm.process_and_validate_instruction(&ix, &accounts, &[Check::all_rent_exempt()])
    });

    raw_result.unwrap();
    let acc = &resulting_accounts[ACC_IDX].1;
    assert_eq!(PROG_ID, acc.owner);

    save_cus_to_file("basic", compute_units_consumed);
}

const PK_EXCL: [[u8; 32]; 2] = [[0; 32], PROG_ID.to_bytes()];

proptest! {
    #[test]
    fn rent_lamports_matches_sol_default(
        [payer, acc] in two_different_pubkeys(),

        // CreateAccount via CPI is limited to realloc data limit,
        // not MAX_PERMITTED_DATA_LENGTH
        // https://stackoverflow.com/a/70156099/5057425.
        //
        // There is no reason to CPI allocate() anymore? since you can just
        // realloc after assigning to yourself
        size in 0usize..=1024 * 10,
    ) {
        silence_mollusk_prog_logs();

        for pk in [payer, acc] {
            if PK_EXCL.contains(&pk) {
                return Ok(());
            }
        }

        let [payer, acc] = [payer, acc].map(Pubkey::new_from_array);

        let (ix, accounts) = setup(payer, acc, size, None);

        let InstructionResult {
            raw_result,
            resulting_accounts,
            ..
        } = SVM.with(|svm| svm.process_and_validate_instruction(
            &ix,
            &accounts,
            &[Check::all_rent_exempt()]
        ));

        raw_result.unwrap();
        let acc = &resulting_accounts[ACC_IDX].1;
        prop_assert_eq!(PROG_ID, acc.owner);
    }
}
