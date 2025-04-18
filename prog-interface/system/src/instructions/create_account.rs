use generic_array_struct::generic_array_struct;
use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::{internal_utils::signer_writable_to_perms, Instruction};

pub const CREATE_ACCOUNT_IX_DISCM: [u8; 4] = [0; 4];

#[generic_array_struct(pub)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CreateAccountIxAccs<T> {
    pub funding: T,
    pub new: T,
}

impl<T: Copy> CreateAccountIxAccs<T> {
    #[inline]
    pub const fn memset(val: T) -> Self {
        Self([val; CREATE_ACCOUNT_IX_ACCS_LEN])
    }
}

pub type CreateAccountIxAccounts<'a> = CreateAccountIxAccs<AccountHandle<'a>>;
pub type CreateAccountIxAccsFlag = CreateAccountIxAccs<bool>;
pub type CreateAccountIxAccountPerms = CreateAccountIxAccs<AccountPerms>;

pub const CREATE_ACCOUNT_IX_IS_SIGNER: CreateAccountIxAccsFlag =
    CreateAccountIxAccsFlag::memset(true);

pub const CREATE_ACCOUNT_IX_IS_WRITABLE: CreateAccountIxAccsFlag =
    CreateAccountIxAccsFlag::memset(true);

pub const CREATE_ACCOUNT_IX_ACCOUNT_PERMS: CreateAccountIxAccountPerms =
    CreateAccountIxAccs(signer_writable_to_perms(
        CREATE_ACCOUNT_IX_IS_SIGNER.0,
        CREATE_ACCOUNT_IX_IS_WRITABLE.0,
    ));

pub const CREATE_ACCOUNT_IX_DATA_LEN: usize = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CreateAccountIxData([u8; CREATE_ACCOUNT_IX_DATA_LEN]);

impl CreateAccountIxData {
    #[inline]
    pub fn new(lamports: u64, space: usize, owner: &[u8; 32]) -> Self {
        let mut ix_data = [0u8; 52];
        ix_data[0..4].copy_from_slice(&CREATE_ACCOUNT_IX_DISCM);
        ix_data[4..12].copy_from_slice(&lamports.to_le_bytes());
        ix_data[12..20].copy_from_slice(&(space as u64).to_le_bytes());
        ix_data[20..].copy_from_slice(owner);

        Self(ix_data)
    }

    #[inline]
    pub fn as_buf(&self) -> &[u8; CREATE_ACCOUNT_IX_DATA_LEN] {
        &self.0
    }
}

#[inline]
pub fn create_account_ix<'account, 'data>(
    system_prog: AccountHandle<'account>,
    accounts: CreateAccountIxAccounts<'account>,
    ix_data: &'data CreateAccountIxData,
) -> Instruction<'account, 'data, CREATE_ACCOUNT_IX_ACCS_LEN> {
    Instruction {
        prog: system_prog,
        data: ix_data.as_buf(),
        accounts: accounts
            .0
            .into_iter()
            .zip(CREATE_ACCOUNT_IX_ACCOUNT_PERMS.0),
    }
}
