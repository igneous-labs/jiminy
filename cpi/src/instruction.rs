use core::mem::MaybeUninit;

use jiminy_account::AccountHandle;

use crate::{AccountPerms, MAX_CPI_ACCOUNT_INFOS, ONE_KB};

/// A CPI instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instr<'a, 'account> {
    pub prog: AccountHandle<'account>,
    pub data: &'a [u8],
    pub accounts: &'a [(AccountHandle<'account>, AccountPerms)],
}

/// An owned [`Instr`] backed by constant sized arrays for data and accounts
#[derive(Debug, Clone, Copy)]
pub struct Instruction<
    'account,
    const MAX_DATA_LEN: usize = ONE_KB,
    const MAX_ACCOUNTS_LEN: usize = MAX_CPI_ACCOUNT_INFOS,
> {
    accounts: [MaybeUninit<(AccountHandle<'account>, AccountPerms)>; MAX_ACCOUNTS_LEN],
    data: [MaybeUninit<u8>; MAX_DATA_LEN],
    accounts_len: u8,
    data_len: u16,
    prog: AccountHandle<'account>,
}

impl<'account, const MAX_DATA_LEN: usize, const MAX_ACCOUNTS_LEN: usize>
    Instruction<'account, MAX_DATA_LEN, MAX_ACCOUNTS_LEN>
{
    #[inline]
    pub const fn new_empty(prog: AccountHandle<'account>) -> Self {
        const {
            if MAX_DATA_LEN > u16::MAX as usize {
                panic!("MAX_DATA_LEN cannot be > u16::MAX");
            }
            if MAX_ACCOUNTS_LEN > u8::MAX as usize {
                panic!("MAX_ACCOUNTS_LEN cannot be > u8::MAX")
            }
        };

        const UNINIT_ACCOUNT: MaybeUninit<(AccountHandle<'_>, AccountPerms)> =
            MaybeUninit::uninit();
        const UNINIT_DATA: MaybeUninit<u8> = MaybeUninit::uninit();

        Self {
            accounts: [UNINIT_ACCOUNT; MAX_ACCOUNTS_LEN],
            data: [UNINIT_DATA; MAX_DATA_LEN],
            accounts_len: 0,
            data_len: 0,
            prog,
        }
    }

    /// Returns `Err(bytes)` if self.data does not have enough capacity
    #[inline]
    pub fn extend_data_from_slice<'a>(&mut self, bytes: &'a [u8]) -> Result<(), &'a [u8]> {
        if bytes.len() > MAX_DATA_LEN - self.data_len() {
            Err(bytes)
        } else {
            unsafe {
                self.extend_data_from_slice_unchecked(bytes);
            }
            Ok(())
        }
    }

    /// # Safety
    /// - [`self`] must have enough space left in `self.data` to fit the entire `slice`
    #[inline]
    pub unsafe fn extend_data_from_slice_unchecked(&mut self, bytes: &[u8]) {
        // non-overlapping because &mut self guarantees no one else has access to self.data
        bytes
            .as_ptr()
            .copy_to_nonoverlapping(self.data.as_mut_ptr().cast(), bytes.len());
        // as-safety: if we have enough space, then self.data_len + slice.len() must be <= u16::MAX
        // since max possible MAX_DATA_LEN is u16::MAX
        self.data_len += bytes.len() as u16;
    }

    /// Returns `Err(accounts)` if self.accounts does not have enough capacity
    #[inline]
    pub fn extend_accounts_from_slice<'a>(
        &mut self,
        accounts: &'a [(AccountHandle<'account>, AccountPerms)],
    ) -> Result<(), &'a [(AccountHandle<'account>, AccountPerms)]> {
        if accounts.len() > MAX_ACCOUNTS_LEN - self.accounts_len() {
            Err(accounts)
        } else {
            unsafe {
                self.extend_accounts_from_slice_unchecked(accounts);
            }
            Ok(())
        }
    }

    /// # Safety
    /// - [`self`] must have enough space left in `self.accounts` to fit the entire `slice`
    #[inline]
    pub unsafe fn extend_accounts_from_slice_unchecked(
        &mut self,
        accounts: &[(AccountHandle<'account>, AccountPerms)],
    ) {
        // non-overlapping because &mut self guarantees no one else has access to self.data
        accounts
            .as_ptr()
            .copy_to_nonoverlapping(self.accounts.as_mut_ptr().cast(), accounts.len());
        // as-safety: if we have enough space, then self.accounts_len + slice.len() must be <= u8::MAX
        // since max possible MAX_ACCOUNTS_LEN is u8::MAX
        self.accounts_len += accounts.len() as u8;
    }

    /// Returns None if `data.len() > MAX_DATA_LEN` or `accounts.len() > MAX_ACCOUNTS_LEN`
    #[inline]
    pub fn new(
        prog: AccountHandle<'account>,
        data: &[u8],
        accounts: &[(AccountHandle<'account>, AccountPerms)],
    ) -> Option<Self> {
        let mut res = Self::new_empty(prog);
        res.extend_data_from_slice(data).ok()?;
        res.extend_accounts_from_slice(accounts).ok()?;
        Some(res)
    }

    /// # Safety
    /// - `data.len() <= MAX_DATA_LEN`
    /// - `accounts.len() <= MAX_ACCOUNTS_LEN`
    #[inline]
    pub unsafe fn new_unchecked(
        prog: AccountHandle<'account>,
        data: &[u8],
        accounts: &[(AccountHandle<'account>, AccountPerms)],
    ) -> Self {
        let mut res = Self::new_empty(prog);
        res.extend_data_from_slice_unchecked(data);
        res.extend_accounts_from_slice_unchecked(accounts);
        res
    }

    #[inline]
    pub const fn data_len_u16(&self) -> u16 {
        self.data_len
    }

    #[inline]
    pub const fn data_len(&self) -> usize {
        self.data_len_u16() as usize
    }

    #[inline]
    pub const fn is_data_full(&self) -> bool {
        self.data_len() == MAX_DATA_LEN
    }

    #[inline]
    pub const fn accounts_len_u8(&self) -> u8 {
        self.accounts_len
    }

    #[inline]
    pub const fn accounts_len(&self) -> usize {
        self.accounts_len_u8() as usize
    }

    #[inline]
    pub const fn is_accounts_full(&self) -> bool {
        self.accounts_len() == MAX_ACCOUNTS_LEN
    }

    #[inline]
    pub const fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data.as_ptr().cast(), self.data_len()) }
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.data_len()) }
    }

    #[inline]
    pub const fn accounts(&self) -> &[(AccountHandle<'account>, AccountPerms)] {
        unsafe { core::slice::from_raw_parts(self.accounts.as_ptr().cast(), self.accounts_len()) }
    }

    #[inline]
    pub fn accounts_mut(&mut self) -> &mut [(AccountHandle<'account>, AccountPerms)] {
        unsafe {
            core::slice::from_raw_parts_mut(self.accounts.as_mut_ptr().cast(), self.accounts_len())
        }
    }

    #[inline]
    pub const fn prog(&self) -> AccountHandle<'account> {
        self.prog
    }

    #[inline]
    pub fn prog_mut(&mut self) -> &mut AccountHandle<'account> {
        &mut self.prog
    }

    #[inline]
    pub const fn as_instr(&self) -> Instr<'_, 'account> {
        Instr {
            prog: self.prog,
            data: self.data(),
            accounts: self.accounts(),
        }
    }
}
