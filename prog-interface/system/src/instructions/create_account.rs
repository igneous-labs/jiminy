use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::Instruction;

pub const CREATE_ACCOUNT_IX_DISCM: [u8; 4] = [0; 4];

#[derive(Debug, Clone, Copy)]
pub struct CreateAccountAccounts<'account> {
    pub funding: AccountHandle<'account>,
    pub new: AccountHandle<'account>,
}

#[derive(Debug, Clone, Copy)]
pub struct CreateAccountIxArgs {
    pub lamports: u64,
    pub space: u64,
    pub owner: [u8; 32],
}

#[inline]
pub fn create_account_ix<'account>(
    system_prog: AccountHandle<'account>,
    CreateAccountAccounts { funding, new }: CreateAccountAccounts<'account>,
    CreateAccountIxArgs {
        lamports,
        space,
        owner,
    }: CreateAccountIxArgs,
) -> Instruction<'account> {
    let mut ix_data = [0u8; 52];
    ix_data[0..4].copy_from_slice(&CREATE_ACCOUNT_IX_DISCM);
    ix_data[4..12].copy_from_slice(&lamports.to_le_bytes());
    ix_data[12..20].copy_from_slice(&space.to_le_bytes());
    ix_data[20..].copy_from_slice(&owner);

    unsafe {
        Instruction::new_unchecked(
            system_prog,
            &ix_data,
            &[
                (
                    funding,
                    AccountPerms {
                        is_signer: true,
                        is_writable: true,
                    },
                ),
                (
                    new,
                    AccountPerms {
                        is_signer: true,
                        is_writable: true,
                    },
                ),
            ],
        )
    }
}
