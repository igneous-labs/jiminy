use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::Instruction;

pub const TRANSFER_IX_DISCM: [u8; 4] = [2, 0, 0, 0];

#[derive(Debug, Clone, Copy)]
pub struct TransferAccounts<'account> {
    pub from: AccountHandle<'account>,
    pub to: AccountHandle<'account>,
}

#[inline]
pub fn transfer_ix<'account>(
    system_prog: AccountHandle<'account>,
    TransferAccounts { from, to }: TransferAccounts<'account>,
    lamports: u64,
) -> Instruction<'account> {
    let mut ix_data = [0u8; 12];
    ix_data[0..4].copy_from_slice(&TRANSFER_IX_DISCM);
    ix_data[4..].copy_from_slice(&lamports.to_le_bytes());

    unsafe {
        Instruction::new_unchecked(
            system_prog,
            &ix_data,
            &[
                (
                    from,
                    AccountPerms {
                        is_signer: true,
                        is_writable: true,
                    },
                ),
                (
                    to,
                    AccountPerms {
                        is_signer: false,
                        is_writable: true,
                    },
                ),
            ],
        )
    }
}
