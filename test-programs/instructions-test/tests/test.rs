//! .so file size: 8264

//#![cfg(feature = "test-sbf")]

use instructions_test::IxArgs;
use mollusk_svm::{result::InstructionResult, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, BorrowedAccountMeta, BorrowedInstruction, Instruction};
use solana_instructions_sysvar::serialize_instructions;
use solana_pubkey::Pubkey;

const PROG_NAME: &str = "instructions_test";
const PROG_ID: Pubkey = solana_pubkey::pubkey!("GRQbyvXVpwuQdRTHuVLzYLX7zoduu9caY73mQM8vL6jA");

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

/// CUs: 141
#[test]
fn instructions_test_basic_no_other_ixs_cus() {
    let svm = Mollusk::new(&PROG_ID, PROG_NAME);

    let ix = instr(&IxArgs {
        ixs_len: 1,
        ix_idx: 0,
        ix_data_len: core::mem::size_of::<IxArgs>().try_into().unwrap(),
        acc_idx: 0,
        pubkey: jiminy_sysvar_instructions::ID,
        is_writable: 0,
        is_signer: 0,
    });
    let ixs = &[ix];
    let ixs_sysvar = instructions_sysvar(ixs);

    let InstructionResult {
        raw_result,
        compute_units_consumed,
        ..
    } = svm.process_instruction_chain(ixs, &[ixs_sysvar]);

    raw_result.unwrap();

    eprintln!("{compute_units_consumed} CUs");
}

// mollusk doesnt provide a way to auto create the sysvar as the runtime would right now
// so we have to create it manually
fn instructions_sysvar(instructions: &[Instruction]) -> (Pubkey, Account) {
    let data = serialize_instructions(
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

    (
        Pubkey::new_from_array(jiminy_sysvar_instructions::ID),
        Account {
            data,
            ..Default::default()
        },
    )
}
