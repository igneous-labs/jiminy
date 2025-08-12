//! .so file size: 2608

#![cfg(feature = "test-sbf")]

use jiminy_test_utils::silence_mollusk_prog_logs;
use mollusk_svm::{program::keyed_account_for_system_program, result::InstructionResult, Mollusk};
use proptest::prelude::*;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "cpi_sys_transfer";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");

/// CUs: 1315
#[test]
fn transfer_basic_cus() {
    const TRF_AMT: u64 = 1_000_000_000;

    let from_pk = solana_pubkey::pubkey!("FpaavSQvEQhPDoQoLUHhmBsKZsG2WJQXj7FBCSPE1TZ1");
    let from = Account {
        lamports: TRF_AMT,
        data: vec![],
        owner: solana_system_program::id(),
        executable: false,
        rent_epoch: u64::MAX,
    };
    let to_pk = solana_pubkey::pubkey!("9diwgHx6xrDjrvXUVx8B4drJMzv9ddh9fBSx59EWjFPU");
    let to = Account {
        lamports: 0,
        data: vec![],
        owner: solana_system_program::id(),
        executable: false,
        rent_epoch: u64::MAX,
    };

    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let InstructionResult {
        compute_units_consumed,
        raw_result,
        resulting_accounts,
        ..
    } = svm.process_instruction(
        &Instruction::new_with_bytes(
            PROG_ID,
            &TRF_AMT.to_le_bytes(),
            vec![
                AccountMeta {
                    pubkey: solana_system_program::id(),
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: from_pk,
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: to_pk,
                    is_signer: false,
                    is_writable: true,
                },
            ],
        ),
        &[
            keyed_account_for_system_program(),
            (from_pk, from),
            (to_pk, to),
        ],
    );

    raw_result.unwrap();
    eprintln!("{compute_units_consumed} CUs");

    assert_eq!(resulting_accounts[1].1.lamports, 0);
    assert_eq!(resulting_accounts[2].1.lamports, TRF_AMT);
}

prop_compose! {
    fn valid_trf_balances()
        (amt in any::<u64>())
        (amt in Just(amt), from in amt.., to in 0..=u64::MAX-amt) -> [u64; 3] {
            [amt, from, to]
        }
}

prop_compose! {
    fn two_different_pubkeys()
        (p1l in any::<u128>(), p1h in any::<u128>())
        (
            p1l in Just(p1l),
            p1h in Just(p1h),
            p2l in any::<u128>(),
            p2h in (0..=p1h.saturating_sub(1)).prop_union(p1h.saturating_add(1)..=u128::MAX)
        ) -> [Pubkey; 2] {
            [(p1l, p1h), (p2l, p2h)].map(|(l, h)| {
                let mut buf = [0u8; 32];
                buf[..16].copy_from_slice(&l.to_le_bytes());
                buf[16..].copy_from_slice(&h.to_le_bytes());
                Pubkey::new_from_array(buf)
            })
        }
}

proptest! {
    #[test]
    fn transfers(
        [amt, from_amt, to_amt] in valid_trf_balances(),
        [from_pk, to_pk] in two_different_pubkeys(),
        prog_id: [u8; 32],
    ) {
        let prog_id = Pubkey::new_from_array(prog_id);
        let [from, to] = [from_amt, to_amt].map(|amt| Account {
            lamports: amt,
            data: vec![],
            owner: solana_system_program::id(),
            executable: false,
            rent_epoch: u64::MAX,
        });

        let svm = Mollusk::new(&prog_id, PROG_NAME);
        silence_mollusk_prog_logs();

        let InstructionResult {
            raw_result,
            resulting_accounts,
            ..
        } = svm.process_instruction(
            &Instruction::new_with_bytes(
                prog_id,
                &amt.to_le_bytes(),
                vec![
                    AccountMeta {
                        pubkey: solana_system_program::id(),
                        is_signer: false,
                        is_writable: false,
                    },
                    AccountMeta {
                        pubkey: from_pk,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: to_pk,
                        is_signer: false,
                        is_writable: true,
                    },
                ],
            ),
            &[
                keyed_account_for_system_program(),
                (from_pk, from),
                (to_pk, to),
            ],
        );

        prop_assert_eq!(raw_result, Ok(()));

        prop_assert_eq!(resulting_accounts[1].1.lamports, from_amt - amt);
        prop_assert_eq!(resulting_accounts[2].1.lamports, to_amt + amt);
    }
}
