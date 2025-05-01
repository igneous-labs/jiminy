//! .so file size: 8272

#![cfg(feature = "test-sbf")]

use std::{cmp::min, collections::HashMap};

use echo_return_data::MAX_ACCS;
use jiminy_return_data::MAX_RETURN_DATA;
use jiminy_test_utils::silence_mollusk_prog_logs;
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sdk_ids::bpf_loader_upgradeable;

const PROG_NAME: &str = "echo_return_data";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("FpaavSQvEQhPDoQoLUHhmBsKZsG2WJQXj7FBCSPE1TZ1");

/// CUs: 478
#[test]
fn entrypoint_basic_cus() {
    let a1_is_exec = false;
    let a1_pk = solana_pubkey::pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");
    let a1 = Account {
        lamports: 100_000_000,
        data: vec![0, 1, 2],
        owner: solana_system_program::id(),
        executable: a1_is_exec,
        rent_epoch: u64::MAX,
    };
    let a1_meta = AccountMeta {
        pubkey: a1_pk,
        is_signer: false,
        is_writable: true,
    };

    let a2_is_exec = true;
    let a2_pk = solana_pubkey::pubkey!("9diwgHx6xrDjrvXUVx8B4drJMzv9ddh9fBSx59EWjFPU");
    let a2 = Account {
        lamports: 100_000_000,
        data: Vec::new(),
        owner: bpf_loader_upgradeable::ID,
        executable: a2_is_exec,
        rent_epoch: u64::MAX,
    };
    let a2_meta = AccountMeta {
        pubkey: a2_pk,
        is_signer: false,
        is_writable: false,
    };
    let ix_data = &[1];
    let metas = vec![a1_meta.clone(), a2_meta.clone()];
    let n_accounts = metas.len();

    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        return_data,
        ..
    } = svm.process_instruction(
        &Instruction::new_with_bytes(PROG_ID, ix_data, metas),
        &[(a1_pk, a1), (a2_pk, a2)],
    );

    raw_result.unwrap();

    eprintln!("{compute_units_consumed} CUs");

    for (i, (meta, is_exec)) in [(a1_meta, a1_is_exec), (a2_meta, a2_is_exec)]
        .iter()
        .enumerate()
    {
        let start = i * 35;
        assert_eq!(&return_data[start..start + 32], meta.pubkey.to_bytes());
        assert_eq!(return_data[start + 32], if *is_exec { 1 } else { 0 });
        assert_eq!(return_data[start + 33], if meta.is_signer { 1 } else { 0 });
        assert_eq!(
            return_data[start + 34],
            if meta.is_writable { 1 } else { 0 }
        );
    }

    let ix_data_start = n_accounts * 35;
    assert_eq!(
        &return_data[ix_data_start..ix_data_start + ix_data.len()],
        ix_data
    );

    let ret_data_prog_id_start = ix_data_start + ix_data.len();
    assert_eq!(
        &return_data[ret_data_prog_id_start..ret_data_prog_id_start + 32],
        PROG_ID.to_bytes()
    );
}

#[test]
fn entrypoint_accounts_data_empty() {
    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let ix = Instruction::new_with_bytes(PROG_ID, &[], vec![]);
    let InstructionResult {
        raw_result,
        return_data,
        ..
    } = svm.process_instruction(&ix, &[]);
    raw_result.unwrap();

    assert_eq!(&return_data[..32], PROG_ID.to_bytes());
}

proptest! {
    #[test]
    fn entrypoint_deser_correct(
        acc_raw in proptest::collection::vec(any::<[u8; 35]>(), 0..11),
        acc_data in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..2048), 0..11),
        ix_data in proptest::collection::vec(any::<u8>(), 0..1232),
        prog_id: [u8; 32],
    ) {
        let prog_id = Pubkey::new_from_array(prog_id);
        // worst case 64 bytes for sig + 33 bytes for pk and index
        let max_data_len = min(1232 - 97 * (1 + acc_raw.len()), ix_data.len());
        let ix_data = &ix_data[..max_data_len];

        let svm = Mollusk::new(&prog_id, PROG_NAME);
        silence_mollusk_prog_logs();

        let mut account_metas = vec![];
        let mut accounts = vec![];

        for (ar, ad) in acc_raw.iter().zip(acc_data.iter().chain(std::iter::repeat(&Vec::new()))) {
            let pk = Pubkey::new_from_array(ar[..32].try_into().unwrap());
            account_metas.push(AccountMeta {
                pubkey: pk,
                is_signer: ar[33] != 0,
                is_writable: ar[34] != 0,
            });
            accounts.push((pk, Account {
                lamports: 100_000_000,
                data: ad.clone(),
                owner: solana_system_program::id(),
                executable: ar[32] != 0,
                rent_epoch: u64::MAX,
            }));
        }

        let ix = Instruction::new_with_bytes(
            prog_id,
            ix_data,
            account_metas.clone(),
        );
        let InstructionResult {
            raw_result,
            return_data,
            ..
        } = svm.process_instruction(
            &ix,
            &accounts,
        );
        raw_result.unwrap();

        let acc_raw_saved_len = min(acc_raw.len(), MAX_ACCS);
        let acc_raw = &acc_raw[..acc_raw_saved_len];

        let mut highest_priv = HashMap::new();
        for meta in account_metas {
            let v = highest_priv.entry(meta.pubkey).or_insert((meta.is_signer, meta.is_writable));
            if meta.is_signer {
                v.0 = true;
            }
            if meta.is_writable {
                v.1 = true;
            }
        }

        for (i, ar) in acc_raw.iter().enumerate() {
            let start = i * 35;
            let pk = Pubkey::try_from(&ar[..32]).unwrap();
            let (is_signer, is_writable) = highest_priv.get(&pk).unwrap();
            prop_assert_eq!(&return_data[start..start + 32], pk.to_bytes());
            prop_assert_eq!(return_data[start + 32], if ar[32] != 0 { 1 } else { 0 }); // exec
            prop_assert_eq!(return_data[start + 33], if *is_signer { 1 } else { 0 });
            prop_assert_eq!(return_data[start + 34], if *is_writable { 1 } else { 0 });
        }

        // ix data
        let ix_data_start = acc_raw.len() * 35;
        let data_len_truncated = min(MAX_RETURN_DATA.saturating_sub(ix_data_start).saturating_sub(32), ix_data.len());

        prop_assert_eq!(&return_data[ix_data_start..ix_data_start + data_len_truncated], &ix_data[..data_len_truncated]);
        // prog id
        let ret_data_prog_id_start = ix_data_start + data_len_truncated;
        prop_assert_eq!(&return_data[ret_data_prog_id_start..ret_data_prog_id_start + 32], prog_id.to_bytes());
    }
}
