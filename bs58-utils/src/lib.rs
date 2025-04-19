#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

use core::{cmp::Ordering, fmt::Display, ops::Deref};

use bs58::encode::EncodeTarget;

pub type PubkeyStr = Bs58Str<44>;

pub type SignatureStr = Bs58Str<88>;

/// A constant max-size base58-encoded string
/// for encoding of fixed-size buffers
#[derive(Debug, Clone, Copy)]
pub struct Bs58Str<const MAX_STR_LEN: usize> {
    len: usize,

    // dont use MaybeUninit because `EncodeTarget::encode_with` requires &mut [u8],
    // and it is UB to make a ref to uninitialized data
    buf: [u8; MAX_STR_LEN],
}

impl<const MAX_STR_LEN: usize> Bs58Str<MAX_STR_LEN> {
    // formula: https://stackoverflow.com/a/59590236/5057425
    pub const BUF_LEN: usize = { (MAX_STR_LEN * 100).div_ceil(138) };

    // Need to use a const generic with comptime assertion
    // here instead of associated const
    // because we cant do `buf: &[u8; Self::BUF_LEN]` yet
    #[inline]
    pub fn of<const BUF_LEN: usize>(buf: &[u8; BUF_LEN]) -> Self {
        const {
            assert!(BUF_LEN == Self::BUF_LEN);
        }

        let mut res = Self::new();
        // safety: len checked at compile time above
        unsafe {
            bs58::encode(buf).onto(&mut res).unwrap_unchecked();
        }
        res
    }

    // Need to use a const generic with comptime assertion
    // here instead of associated const
    // because we cant do `-> [u8; Self::BUF_LEN]` yet
    #[inline]
    pub fn decode<const BUF_LEN: usize>(&self) -> [u8; BUF_LEN] {
        const {
            assert!(BUF_LEN == Self::BUF_LEN);
        }

        let mut res = [0u8; BUF_LEN];
        // safety: len checked at compile time above
        unsafe {
            bs58::decode(self.as_bytes())
                .onto(&mut res)
                .unwrap_unchecked()
        };
        res
    }

    #[inline]
    pub const fn new() -> Self {
        Self {
            buf: [0u8; MAX_STR_LEN],
            len: 0,
        }
    }

    #[inline]
    pub const fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr(), self.len) }
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        // safety: bs58 alphabet is valid ascii/utf8
        unsafe { core::str::from_utf8_unchecked(self.as_slice()) }
    }
}

impl<const MAX_STR_LEN: usize> EncodeTarget for Bs58Str<MAX_STR_LEN> {
    fn encode_with(
        &mut self,
        _max_len: usize,
        f: impl for<'a> FnOnce(&'a mut [u8]) -> bs58::encode::Result<usize>,
    ) -> bs58::encode::Result<usize> {
        let len = f(&mut self.buf)?;
        if len > MAX_STR_LEN {
            Err(bs58::encode::Error::BufferTooSmall)
        } else {
            self.len = len;
            Ok(len)
        }
    }
}

impl<const MAX_STR_LEN: usize> Deref for Bs58Str<MAX_STR_LEN> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const MAX_STR_LEN: usize> Default for Bs58Str<MAX_STR_LEN> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_STR_LEN: usize> PartialEq for Bs58Str<MAX_STR_LEN> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const MAX_STR_LEN: usize> Eq for Bs58Str<MAX_STR_LEN> {}

impl<const MAX_STR_LEN: usize> Ord for Bs58Str<MAX_STR_LEN> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<const MAX_STR_LEN: usize> PartialOrd for Bs58Str<MAX_STR_LEN> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const MAX_STR_LEN: usize> core::hash::Hash for Bs58Str<MAX_STR_LEN> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl<const MAX_STR_LEN: usize> Display for Bs58Str<MAX_STR_LEN> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn pubkey_round_trip(pk: [u8; 32]) {
            let encoded = PubkeyStr::of(&pk);
            let decoded = encoded.decode();
            prop_assert_eq!(decoded, pk);
        }
    }

    proptest! {
        #[test]
        fn check_pubkey_against_bs58_impl(pk: [u8; 32]) {
            let encoded = PubkeyStr::of(&pk);
            let bs58_impl = bs58::encode(pk).into_string();
            assert_eq!(bs58_impl.as_str(), encoded.as_str());
        }
    }
}
