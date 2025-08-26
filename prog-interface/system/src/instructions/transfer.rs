use generic_array_struct::generic_array_struct;
use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::{internal_utils::signer_writable_to_perms, AccountHandlePerms};

pub const TRANSFER_IX_DISCM: [u8; 4] = [2, 0, 0, 0];

#[generic_array_struct(builder pub)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TransferIxAccs<T> {
    pub from: T,
    pub to: T,
}

impl<T: Copy> TransferIxAccs<T> {
    #[inline]
    pub const fn memset(val: T) -> Self {
        Self([val; TRANSFER_IX_ACCS_LEN])
    }
}

pub type TransferIxAccounts<'a> = TransferIxAccs<AccountHandle<'a>>;
pub type TransferIxAccsFlag = TransferIxAccs<bool>;
pub type TransferIxAccountPerms = TransferIxAccs<AccountPerms>;

pub const TRANSFER_IX_IS_SIGNER: TransferIxAccsFlag =
    TransferIxAccs::memset(false).const_with_from(true);

pub const TRANSFER_IX_IS_WRITABLE: TransferIxAccsFlag = TransferIxAccs::memset(true);

pub const TRANSFER_IX_ACCOUNT_PERMS: TransferIxAccountPerms = TransferIxAccs(
    signer_writable_to_perms(TRANSFER_IX_IS_SIGNER.0, TRANSFER_IX_IS_WRITABLE.0),
);

pub const TRANSFER_IX_DATA_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TransferIxData([u8; TRANSFER_IX_DATA_LEN]);

impl TransferIxData {
    #[inline]
    pub fn new(lamports: u64) -> Self {
        let mut ix_data = [0u8; 12];
        ix_data[0..4].copy_from_slice(&TRANSFER_IX_DISCM);
        ix_data[4..].copy_from_slice(&lamports.to_le_bytes());

        Self(ix_data)
    }

    #[inline]
    pub fn as_buf(&self) -> &[u8; TRANSFER_IX_DATA_LEN] {
        &self.0
    }
}

impl<'accounts> TransferIxAccounts<'accounts> {
    #[inline]
    pub fn into_account_handle_perms(self) -> AccountHandlePerms<'accounts, TRANSFER_IX_ACCS_LEN> {
        self.0.into_iter().zip(TRANSFER_IX_ACCOUNT_PERMS.0)
    }
}
