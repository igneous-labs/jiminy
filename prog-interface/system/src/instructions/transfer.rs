use generic_array_struct::generic_array_struct;
use jiminy_cpi::{account::AccountHandle, AccountPerms};

use super::{
    internal_utils::{signer_writable_to_perms, zip_accounts_perms},
    Instruction,
};

pub const TRANSFER_IX_DISCM: [u8; 4] = [2, 0, 0, 0];

#[generic_array_struct(pub)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TransferAccs<T> {
    pub from: T,
    pub to: T,
}

pub type TransferAccounts<'a> = TransferAccs<AccountHandle<'a>>;
pub type TransferAccsFlag = TransferAccs<bool>;
pub type TransferAccountPerms = TransferAccs<AccountPerms>;

pub const TRANSFER_IX_IS_SIGNER: TransferAccsFlag =
    TransferAccs([false; TRANSFER_ACCS_LEN]).const_with_from(true);

pub const TRANSFER_IX_IS_WRITABLE: TransferAccsFlag = TransferAccs([true, true]);

pub const TRANSFER_IX_ACCOUNT_PERMS: TransferAccountPerms = TransferAccs(signer_writable_to_perms(
    TRANSFER_IX_IS_SIGNER.0,
    TRANSFER_IX_IS_WRITABLE.0,
));

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

#[inline]
pub fn transfer_ix<'account>(
    system_prog: AccountHandle<'account>,
    accounts: TransferAccounts<'account>,
    ix_data: &TransferIxData,
) -> Instruction<'account> {
    unsafe {
        Instruction::new_unchecked(
            system_prog,
            ix_data.as_buf(),
            &zip_accounts_perms(accounts.0, TRANSFER_IX_ACCOUNT_PERMS.0),
        )
    }
}
