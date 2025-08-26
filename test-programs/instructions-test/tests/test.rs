//! Make sure to build the noop test-program before running tests here.

#![cfg(feature = "test-sbf")]

use std::{cell::RefCell, collections::HashSet};

use instructions_test::IxArgs;
use jiminy_sysvar_instructions::sysvar::OWNER_ID;
use jiminy_test_utils::{save_binsize_to_file, save_cus_to_file, silence_mollusk_prog_logs};
use mollusk_svm::{result::InstructionResult, Mollusk};
use proptest::{collection::vec, prelude::*};
use solana_account::Account;
use solana_instruction::{AccountMeta, BorrowedAccountMeta, BorrowedInstruction, Instruction};
use solana_instructions_sysvar::construct_instructions_data;
use solana_pubkey::Pubkey;
use solana_sdk_ids::bpf_loader_upgradeable;

const PROG_NAME: &str = "instructions_test";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("GRQbyvXVpwuQdRTHuVLzYLX7zoduu9caY73mQM8vL6jA");
const NOOP_PROG_NAME: &str = "noop";

thread_local! {
    static SVM: RefCell<Mollusk> = RefCell::new(Mollusk::new(&PROG_ID, PROG_NAME));
}

fn instr(args: &IxArgs) -> Instruction {
    Instruction::new_with_bytes(
        PROG_ID,
        args.as_buf(),
        vec![AccountMeta {
            pubkey: Pubkey::new_from_array(jiminy_sysvar_instructions::ID),
            is_signer: false,
            is_writable: false,
        }],
    )
}

#[test]
fn save_binsize() {
    save_binsize_to_file(PROG_NAME);
}

#[test]
fn instructions_test_basic_no_other_ixs_cus() {
    let curr_idx = 0;
    let ix = instr(&IxArgs {
        ixs_len: 1,
        ix_idx: 0,
        ix_data_len: core::mem::size_of::<IxArgs>().try_into().unwrap(),
        acc_idx: 0,
        curr_idx,
        pubkey: jiminy_sysvar_instructions::ID,
        is_writable: 0,
        is_signer: 0,
    });
    let ixs = &[ix];
    let ixs_sysvar = instructions_sysvar(ixs, curr_idx);

    SVM.with(|svm| {
        let svm = svm.borrow();
        let InstructionResult {
            raw_result,
            compute_units_consumed,
            ..
        } = svm.process_instruction_chain(ixs, &[ixs_sysvar]);
        raw_result.unwrap();
        save_cus_to_file("basic", compute_units_consumed);
    });
}

proptest! {
    #[test]
    fn instructions_all_nonempty_accounts(
        (ixs, accs) in any_test_ix_seq()
    ) {
        silence_mollusk_prog_logs();

        SVM.with(|svm| {
            let mut svm = svm.borrow_mut();
            let mut program_ids = HashSet::new();
            ixs.iter().for_each(|ix| {
                if program_ids.insert(ix.program_id) {
                    svm.add_program(&ix.program_id, NOOP_PROG_NAME, &bpf_loader_upgradeable::ID);
                }
            });

            let InstructionResult {
                raw_result,
                ..
            } = svm.process_instruction_chain(&ixs, &accs);

            raw_result.unwrap();
        });
    }
}

fn any_meta() -> impl Strategy<Value = AccountMeta> {
    (any::<[u8; 32]>(), any::<bool>(), any::<bool>()).prop_map(|(key, is_signer, is_writable)| {
        AccountMeta {
            pubkey: Pubkey::new_from_array(key),
            is_signer,
            is_writable,
        }
    })
}

fn any_ix() -> impl Strategy<Value = Instruction> {
    (
        any::<[u8; 32]>(),
        // have at least 1 account per ix because our program expects at least 1
        vec(any_meta(), 1..42),
        vec(any::<u8>(), 0..512),
    )
        .prop_map(|(program_id, accounts, data)| Instruction {
            program_id: Pubkey::new_from_array(program_id),
            accounts,
            data,
        })
}

fn any_test_ix_seq() -> impl Strategy<Value = (Vec<Instruction>, Vec<(Pubkey, Account)>)> {
    vec(any_ix(), 0..7)
        .prop_flat_map(|ixs| (0..=ixs.len(), 0..=ixs.len(), Just(ixs)))
        .prop_flat_map(|(curr_idx, ix_idx, mut ixs)| {
            // first fill with a dummy instruction
            ixs.insert(
                curr_idx,
                Instruction::new_with_bytes(PROG_ID, &[], Vec::new()),
            );
            (Just(curr_idx), Just(ix_idx), Just(ixs))
        })
        .prop_flat_map(|(curr_idx, ix_idx, ixs)| {
            let ix = if ix_idx == curr_idx {
                Instruction::new_with_bytes(
                    PROG_ID,
                    IxArgs::default().as_buf(),
                    vec![AccountMeta {
                        pubkey: Pubkey::new_from_array(jiminy_sysvar_instructions::ID),
                        is_signer: false,
                        is_writable: false,
                    }],
                )
            } else {
                ixs[ix_idx].clone()
            };
            (
                Just(curr_idx),
                Just(ix_idx),
                Just(ix.data.len()),
                0..ix.accounts.len(),
                Just(ix),
                Just(ixs),
            )
        })
        .prop_flat_map(|(curr_idx, ix_idx, ix_data_len, acc_idx, ix, mut ixs)| {
            let AccountMeta {
                pubkey,
                is_signer,
                is_writable,
            } = &ix.accounts[acc_idx];
            let [is_writable, is_signer] = [is_writable, is_signer].map(|b| if *b { 1 } else { 0 });
            ixs[curr_idx] = instr(&IxArgs {
                ixs_len: ixs.len().try_into().unwrap(),
                curr_idx: curr_idx.try_into().unwrap(),
                ix_idx: ix_idx.try_into().unwrap(),
                ix_data_len: ix_data_len.try_into().unwrap(),
                acc_idx: acc_idx.try_into().unwrap(),
                pubkey: *pubkey.as_array(),
                is_writable,
                is_signer,
            });
            let mut accs: Vec<_> = ixs
                .iter()
                .flat_map(|ix| ix.accounts.iter().map(|m| (m.pubkey, Account::default())))
                .collect();
            accs.sort_by_key(|i| i.0);
            accs.dedup_by_key(|i| i.0);
            *accs
                .iter_mut()
                .find(|a| a.0 == Pubkey::new_from_array(jiminy_sysvar_instructions::ID))
                .unwrap() = instructions_sysvar(&ixs, curr_idx.try_into().unwrap());

            (Just(ixs), Just(accs))
        })
}

// mollusk doesnt provide a way to auto create the sysvar as the runtime would right now
// so we have to create it manually
fn instructions_sysvar(instructions: &[Instruction], curr_idx: u16) -> (Pubkey, Account) {
    let mut data = construct_instructions_data(
        instructions
            .iter()
            .map(|instruction| BorrowedInstruction {
                program_id: &instruction.program_id,
                accounts: instruction
                    .accounts
                    .iter()
                    .map(|meta| BorrowedAccountMeta {
                        pubkey: &meta.pubkey,
                        is_signer: meta.is_signer,
                        is_writable: meta.is_writable,
                    })
                    .collect(),
                data: &instruction.data,
            })
            .collect::<Vec<_>>()
            .as_slice(),
    );
    *data.split_last_chunk_mut().unwrap().1 = curr_idx.to_le_bytes();

    (
        Pubkey::new_from_array(jiminy_sysvar_instructions::ID),
        Account {
            data,
            owner: Pubkey::new_from_array(OWNER_ID),
            ..Default::default()
        },
    )
}
