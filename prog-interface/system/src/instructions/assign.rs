use generic_array_struct::generic_array_struct;
use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::{
    internal_utils::{signer_writable_to_perms, zip_accounts_perms},
    Instruction,
};

pub const ASSIGN_IX_DISCM: [u8; 4] = [1, 0, 0, 0];

#[generic_array_struct(pub)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AssignAccs<T> {
    pub assign: T,
}

pub type AssignAccounts<'a> = AssignAccs<AccountHandle<'a>>;
pub type AssignAccsFlag = AssignAccs<bool>;
pub type AssignAccountPerms = AssignAccs<AccountPerms>;

pub const ASSIGN_IX_IS_SIGNER: AssignAccsFlag = AssignAccs([true]);

pub const ASSIGN_IX_IS_WRITABLE: AssignAccsFlag = AssignAccs([true]);

pub const ASSIGN_IX_ACCOUNT_PERMS: AssignAccountPerms = AssignAccs(signer_writable_to_perms(
    ASSIGN_IX_IS_SIGNER.0,
    ASSIGN_IX_IS_WRITABLE.0,
));

pub const ASSIGN_IX_DATA_LEN: usize = 36;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AssignIxData([u8; ASSIGN_IX_DATA_LEN]);

impl AssignIxData {
    #[inline]
    pub fn new(owner: &[u8; 32]) -> Self {
        let mut ix_data = [0u8; ASSIGN_IX_DATA_LEN];
        ix_data[0..4].copy_from_slice(&ASSIGN_IX_DISCM);
        ix_data[4..].copy_from_slice(owner);

        Self(ix_data)
    }

    #[inline]
    pub fn as_buf(&self) -> &[u8; ASSIGN_IX_DATA_LEN] {
        &self.0
    }
}

#[inline]
pub fn assign_ix<'account>(
    system_prog: AccountHandle<'account>,
    accounts: AssignAccounts<'account>,
    ix_data: &AssignIxData,
) -> Instruction<'account> {
    unsafe {
        Instruction::new_unchecked(
            system_prog,
            ix_data.as_buf(),
            &zip_accounts_perms(accounts.0, ASSIGN_IX_ACCOUNT_PERMS.0),
        )
    }
}
