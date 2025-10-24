#![cfg(feature = "test-sbf")]

use jiminy_test_utils::{save_binsize_to_file, save_cus_to_file, silence_mollusk_prog_logs};
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::{collection::vec, prelude::*};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "append";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("HiFMLzSe5haw7DvXuGRmyRF5S7WjCG5UPcw2JUpED3RM");

thread_local! {
    static SVM: Mollusk = Mollusk::new(&PROG_ID, PROG_NAME);
}

fn instr(slab: Pubkey, data: &[u8]) -> Instruction {
    Instruction::new_with_bytes(
        PROG_ID,
        data,
        vec![AccountMeta {
            pubkey: slab,
            is_signer: false,
            is_writable: true,
        }],
    )
}

fn slab_acc(data: Vec<u8>) -> Account {
    Account {
        lamports: u64::MAX, // all the rent in the world
        data,
        owner: PROG_ID,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

#[test]
fn save_binsize() {
    save_binsize_to_file(PROG_NAME);
}

#[test]
fn append_test_0to8_cus() {
    const VAL: u64 = u64::MAX;

    let slab_pk = solana_pubkey::pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");

    let ix = instr(slab_pk, VAL.to_le_bytes().as_slice());
    let InstructionResult {
        raw_result,
        compute_units_consumed,
        resulting_accounts,
        ..
    } = SVM.with(|svm| svm.process_instruction(&ix, &[(slab_pk, slab_acc(vec![]))]));

    raw_result.unwrap();
    let d: &[u8; 8] = resulting_accounts[0].1.data.as_slice().try_into().unwrap();
    assert_eq!(u64::from_le_bytes(*d), VAL);
    save_cus_to_file("0to8", compute_units_consumed);
}

proptest! {
    #[test]
    fn data_append_success(
        old_data in vec(any::<u8>(), 0..=128),
        append_data in vec(any::<u8>(), 0..=128),
        slab_pk: [u8; 32],
    ) {
        silence_mollusk_prog_logs();

        let slab_pk = Pubkey::new_from_array(slab_pk);

        let ix = instr(slab_pk, &append_data);
        let InstructionResult {
            raw_result,
            resulting_accounts,
            ..
        } = SVM.with(|svm| svm.process_instruction(&ix, &[(slab_pk, slab_acc(old_data.clone()))]));

        raw_result.unwrap();
        prop_assert_eq!(&resulting_accounts[0].1.data, &[old_data, append_data].concat());
    }
}
