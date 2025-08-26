use generic_array_struct::generic_array_struct;
use jiminy_cpi::{account::AccountHandle, AccountPerms};

use crate::AccountHandlePerms;

use super::internal_utils::signer_writable_to_perms;

pub const ASSIGN_IX_DISCM: [u8; 4] = [1, 0, 0, 0];

#[generic_array_struct(builder pub)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AssignIxAccs<T> {
    pub assign: T,
}

impl<T> AssignIxAccs<T> {
    #[inline]
    pub const fn memset(val: T) -> Self {
        Self([val; ASSIGN_IX_ACCS_LEN])
    }
}

pub type AssignIxAccounts<'a> = AssignIxAccs<AccountHandle<'a>>;
pub type AssignIxAccsFlag = AssignIxAccs<bool>;
pub type AssignIxAccountPerms = AssignIxAccs<AccountPerms>;

pub const ASSIGN_IX_IS_SIGNER: AssignIxAccsFlag = AssignIxAccs::memset(true);

pub const ASSIGN_IX_IS_WRITABLE: AssignIxAccsFlag = AssignIxAccs::memset(true);

pub const ASSIGN_IX_ACCOUNT_PERMS: AssignIxAccountPerms = AssignIxAccs(signer_writable_to_perms(
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

impl<'accounts> AssignIxAccounts<'accounts> {
    #[inline]
    pub fn into_account_handle_perms(self) -> AccountHandlePerms<'accounts, ASSIGN_IX_ACCS_LEN> {
        self.0.into_iter().zip(ASSIGN_IX_ACCOUNT_PERMS.0)
    }
}
