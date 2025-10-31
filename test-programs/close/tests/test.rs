#![cfg(feature = "test-sbf")]

use jiminy_sysvar_rent::Rent;
use jiminy_test_utils::{save_binsize_to_file, save_cus_to_file, silence_mollusk_prog_logs};
use mollusk_svm::{
    result::{Check, InstructionResult},
    Mollusk,
};
use proptest::{collection::vec, prelude::*};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "close";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("9dk23J1t3jpwWFFRMNf7jdfgn3H7m5L1uEtZbKWXKKw5");

const SOLANA_RENT_ZERO: u64 = 890_880;

thread_local! {
    static SVM: Mollusk = Mollusk::new(&PROG_ID, PROG_NAME);
}

fn instr(close: Pubkey, refund_rent_to: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        PROG_ID,
        &[],
        vec![
            AccountMeta {
                pubkey: close,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: refund_rent_to,
                is_signer: false,
                is_writable: true,
            },
        ],
    )
}

fn close_acc(data: Vec<u8>, lamports: Option<u64>) -> Account {
    Account {
        lamports: lamports.unwrap_or_else(|| Rent::DEFAULT.min_balance(data.len())),
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

fn test(accs: &[(Pubkey, Account); 2]) -> InstructionResult {
    let [(close_pk, _), (refund_rent_to_pk, _)] = accs;
    let ix = instr(*close_pk, *refund_rent_to_pk);

    let accs = if close_pk != refund_rent_to_pk {
        accs.as_slice()
    } else {
        &accs[0..1]
    };

    let res = SVM
        .with(|svm| svm.process_and_validate_instruction(&ix, accs, &[Check::all_rent_exempt()]));

    res.raw_result.as_ref().unwrap();

    // assert balanced
    let [bef_lamports, aft_lamports] = [accs, res.resulting_accounts.as_slice()]
        .map(|a| a.iter().fold(0, |acc, (_, a)| acc + a.lamports));
    assert_eq!(bef_lamports, aft_lamports);

    // assert close's new owner is system prog
    let close_resulting = &res.resulting_accounts[0].1;
    assert_eq!(close_resulting.owner, Pubkey::default());

    // assert close has no data
    assert!(close_resulting.data.is_empty());

    // assert lamport balances
    if close_pk != refund_rent_to_pk {
        assert_eq!(close_resulting.lamports, 0);
        assert_eq!(res.resulting_accounts[1].1.lamports, aft_lamports);
    } else {
        assert_eq!(close_resulting.lamports, aft_lamports);
    }

    res
}

#[test]
fn close_test_basic_cus() {
    let close = solana_pubkey::pubkey!("CkebHSWNvZ5w9Q3GTivrEomZZmwWFNqPpzVA9NFZxpg8");
    let refund_rent_to = solana_pubkey::pubkey!("HiFMLzSe5haw7DvXuGRmyRF5S7WjCG5UPcw2JUpED3RM");

    let InstructionResult {
        compute_units_consumed,
        ..
    } = test(&[
        (close, close_acc(vec![0u8; 10], None)),
        (
            refund_rent_to,
            Account {
                lamports: SOLANA_RENT_ZERO,
                ..Default::default()
            },
        ),
    ]);

    save_cus_to_file("basic", compute_units_consumed);
}

fn lamports_strat() -> impl Strategy<Value = [u64; 2]> {
    (0..=(u64::MAX - SOLANA_RENT_ZERO))
        .prop_flat_map(|close_lamports| {
            (
                Just(close_lamports),
                SOLANA_RENT_ZERO..=(u64::MAX - close_lamports),
            )
        })
        .prop_map(|(close_lamports, refund_rent_to_lamports)| {
            [close_lamports, refund_rent_to_lamports]
        })
}

proptest! {
    #[test]
    fn close_success(
        close_data in vec(any::<u8>(), 0..=128),
        [close_lamports, refund_rent_to_lamports] in lamports_strat(),
        close_pk: [u8; 32],
        refund_rent_to_pk: [u8; 32], // may be same as close_pk
    ) {
        silence_mollusk_prog_logs();
        test(&[
            (close_pk.into(), close_acc(close_data, Some(close_lamports))),
            (refund_rent_to_pk.into(), Account { lamports: refund_rent_to_lamports, ..Default::default() }),
        ]);
    }
}
