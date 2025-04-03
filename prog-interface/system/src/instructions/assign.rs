use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::Instruction;

pub const ASSIGN_IX_DISCM: [u8; 4] = [1, 0, 0, 0];

#[derive(Debug, Clone, Copy)]
pub struct AssignAccounts<'account> {
    pub assign: AccountHandle<'account>,
}

#[inline]
pub fn assign_ix<'account>(
    system_prog: AccountHandle<'account>,
    AssignAccounts { assign }: AssignAccounts<'account>,
    // with #[inline], it seems like it its the same CUs and program size
    // regardless of whether this pubkey is passed by reference or value
    owner: [u8; 32],
) -> Instruction<'account> {
    let mut ix_data = [0u8; 36];
    ix_data[0..4].copy_from_slice(&ASSIGN_IX_DISCM);
    ix_data[4..].copy_from_slice(&owner);

    unsafe {
        Instruction::new_unchecked(
            system_prog,
            &ix_data,
            &[(
                assign,
                AccountPerms {
                    is_signer: true,
                    is_writable: true,
                },
            )],
        )
    }
}
