use generic_array_struct::generic_array_struct;
use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::{
    internal_utils::{signer_writable_to_perms, zip_accounts_perms},
    Instruction,
};

pub const CREATE_ACCOUNT_IX_DISCM: [u8; 4] = [0; 4];

#[generic_array_struct(pub)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CreateAccountAccs<T> {
    pub funding: T,
    pub new: T,
}

pub type CreateAccountAccounts<'a> = CreateAccountAccs<AccountHandle<'a>>;
pub type CreateAccountAccsFlag = CreateAccountAccs<bool>;
pub type CreateAccountAccountPerms = CreateAccountAccs<AccountPerms>;

pub const CREATE_ACCOUNT_IX_IS_SIGNER: CreateAccountAccsFlag =
    CreateAccountAccs([true; CREATE_ACCOUNT_ACCS_LEN]);

pub const CREATE_ACCOUNT_IX_IS_WRITABLE: CreateAccountAccsFlag =
    CreateAccountAccs([true; CREATE_ACCOUNT_ACCS_LEN]);

pub const CREATE_ACCOUNT_IX_ACCOUNT_PERMS: CreateAccountAccountPerms =
    CreateAccountAccs(signer_writable_to_perms(
        CREATE_ACCOUNT_IX_IS_SIGNER.0,
        CREATE_ACCOUNT_IX_IS_WRITABLE.0,
    ));

#[derive(Debug, Clone, Copy)]
pub struct CreateAccountIxArgs {
    pub lamports: u64,
    pub space: u64,
    pub owner: [u8; 32],
}

pub const CREATE_ACCOUNT_IX_DATA_LEN: usize = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CreateAccountIxData([u8; CREATE_ACCOUNT_IX_DATA_LEN]);

impl CreateAccountIxData {
    #[inline]
    pub fn new(lamports: u64, space: u64, owner: &[u8; 32]) -> Self {
        let mut ix_data = [0u8; 52];
        ix_data[0..4].copy_from_slice(&CREATE_ACCOUNT_IX_DISCM);
        ix_data[4..12].copy_from_slice(&lamports.to_le_bytes());
        ix_data[12..20].copy_from_slice(&space.to_le_bytes());
        ix_data[20..].copy_from_slice(owner);

        Self(ix_data)
    }

    #[inline]
    pub fn as_buf(&self) -> &[u8; CREATE_ACCOUNT_IX_DATA_LEN] {
        &self.0
    }
}

#[inline]
pub fn create_account_ix<'account>(
    system_prog: AccountHandle<'account>,
    accounts: CreateAccountAccounts<'account>,
    ix_data: &CreateAccountIxData,
) -> Instruction<'account> {
    unsafe {
        Instruction::new_unchecked(
            system_prog,
            ix_data.as_buf(),
            &zip_accounts_perms(accounts.0, CREATE_ACCOUNT_IX_ACCOUNT_PERMS.0),
        )
    }
}
