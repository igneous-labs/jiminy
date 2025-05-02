//! ## References
//! - [serialization format of Instructions sysvar](https://github.com/anza-xyz/solana-sdk/blob/691d3064149e732f105d6ac52b80065f09041fb8/instructions-sysvar/src/lib.rs#L84-L129). Just read the code, the comments are messed up.

#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

// Re-exports
pub mod account {
    pub use jiminy_account::*;
}
pub mod program_error {
    pub use jiminy_sysvar::program_error::*;
}
pub mod sysvar {
    pub use jiminy_sysvar::*;
}

use core::{iter::Map, ptr, slice};

use account::Account;
use sysvar::SysvarId;

pub const ID_STR: &str = "Sysvar1nstructions1111111111111111111111111";

pub const ID: [u8; 32] = const_crypto::bs58::decode_pubkey(ID_STR);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Instructions<'a> {
    /// impl notes:
    /// - this lib is meant to only be used onchain. In the onchain context,
    ///   account data is always guaranteed to be 8-byte aligned. So we make
    ///   the same assumption + assume little-endianness and go to town with
    ///   unsafe pointer casting to achieve aligned reads.
    acc_data: &'a [u8],
}

impl SysvarId for Instructions<'_> {
    const ID: [u8; 32] = ID;
}

/// Constructors
impl<'a> Instructions<'a> {
    /// Returns `None` if `acc` is not the instructions sysvar account.
    ///
    /// This is the only way to safely obtain this struct.
    #[inline]
    pub fn try_from_account(acc: &'a Account) -> Option<Self> {
        if *acc.key() == Self::ID {
            Some(Self {
                acc_data: acc.data(),
            })
        } else {
            None
        }
    }
}

/// instructions length
impl Instructions<'_> {
    #[inline]
    pub const fn len_u16(&self) -> &u16 {
        // number of instructions is first 2-bytes LE
        // of account data
        // safety: account data is 8-byte (2-byte) aligned
        unsafe { &*self.acc_data.as_ptr().cast() }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        (*self.len_u16()) as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        *self.len_u16() == 0
    }
}

/// instructions offset table
impl Instructions<'_> {
    const OFFSET_TABLE_OFFSET: usize = 2;

    #[inline]
    const fn offset_table(&self) -> &[u16] {
        // safety:
        // - OFFSET_TABLE_OFFSET position is guaranteed to be 2-byte (u16) aligned
        //   since account data is 8-byte aligned and OFFSET_TABLE_OFFSET = 2
        unsafe {
            slice::from_raw_parts(
                self.acc_data.as_ptr().add(Self::OFFSET_TABLE_OFFSET).cast(),
                self.len(),
            )
        }
    }
}

// instructions

/// An introspected instruction from instructions sysvar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntroInstr<'a> {
    /// subslice of this entire instruction, which spans
    /// from the u16 accounts_len at the start to end
    /// of instruction data at the end
    buf: &'a [u8],
    accounts_len: usize,
    data_len: usize,
}

pub type IntroInstrIter<'a, F> = Map<slice::Iter<'a, u16>, F>;

/// Instruction unpacking
impl Instructions<'_> {
    #[inline]
    pub fn iter<'a>(&'a self) -> IntroInstrIter<'a, impl Fn(&u16) -> IntroInstr<'a> + 'a> {
        let unpack_ix = |offset: &u16| {
            let start = usize::from(*offset);
            let mut end = start;

            // first 2-bytes are ix accounts len.
            // cannot guarantee 2-byte alignment due to arbitrary ix data length
            //
            // index-safety: offset table should give valid offsets
            let accounts_len = match self.acc_data[end..end + 2] {
                [u0, u1] => usize::from(u16::from_le_bytes([u0, u1])),
                _ => unreachable!(),
            };
            end += 2;

            // each account input is 33 bytes:
            // InstructionsAccountPerms + pubkey
            end += accounts_len * INTRO_INSTR_ACC_LEN;

            // next 32 bytes are program ID
            end += 32;

            // next 2 bytes are data_len
            //
            // index-safety: offset table should give valid offsets
            let data_len = match self.acc_data[end..end + 2] {
                [u0, u1] => usize::from(u16::from_le_bytes([u0, u1])),
                _ => unreachable!(),
            };
            end += 2;

            // last bytes are data
            end += data_len;

            IntroInstr {
                buf: &self.acc_data[start..end],
                accounts_len,
                data_len,
            }
        };

        self.offset_table().iter().map(unpack_ix)
    }
}

/// Individual instruction accessors
impl IntroInstr<'_> {
    const ACCOUNTS_OFFSET: usize = 2;

    #[inline]
    pub const fn accounts(&self) -> &[IntroInstrAcc] {
        // safety: IntroInstrAcc has no alignment requirements,
        // data serialized by the runtime should be valid
        unsafe {
            slice::from_raw_parts(
                self.buf.as_ptr().add(Self::ACCOUNTS_OFFSET).cast(),
                self.accounts_len,
            )
        }
    }

    const fn data_offset(&self) -> usize {
        2 // accounts_len
        + INTRO_INSTR_ACC_LEN * self.accounts_len // accounts
        + 32 // program id
        + 2 // data len
    }

    #[inline]
    pub const fn data(&self) -> &[u8] {
        // safety: &[u8] has no alignment requirements,
        // data serialized by the runtime should be valid
        unsafe {
            slice::from_raw_parts(
                self.buf.as_ptr().add(self.data_offset()).cast(),
                self.data_len,
            )
        }
    }
}

const INTRO_INSTR_ACC_LEN: usize = 33;

/// An instruction account of an introspected instruction from the instructions sysvar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IntroInstrAcc([u8; INTRO_INSTR_ACC_LEN]);

impl IntroInstrAcc {
    #[inline]
    pub const fn as_buf(&self) -> &[u8; INTRO_INSTR_ACC_LEN] {
        &self.0
    }

    #[inline]
    pub const fn flags(&self) -> &IntroInstrAccFlags {
        // safety: valid cast bec IntroInstrAccFlags is repr(transparent)
        unsafe { &*ptr::from_ref(&self.0[0]).cast() }
    }

    #[inline]
    pub const fn key(&self) -> &[u8; 32] {
        // safety: self.length is 33, so [1..] is [u8; 32]. align = 1
        unsafe { &*self.0.as_ptr().add(1).cast() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IntroInstrAccFlags(u8);

impl IntroInstrAccFlags {
    const IS_SIGNER: u8 = 0b0000_0001;
    const IS_WRITABLE: u8 = 0b0000_0010;

    #[inline]
    pub const fn as_u8(&self) -> &u8 {
        &self.0
    }

    #[inline]
    pub const fn is_signer(&self) -> bool {
        self.is_flag_set(Self::IS_SIGNER)
    }

    #[inline]
    pub const fn is_writable(&self) -> bool {
        self.is_flag_set(Self::IS_WRITABLE)
    }

    #[inline]
    const fn is_flag_set(&self, flag: u8) -> bool {
        flag & self.0 == flag
    }
}
