#![cfg_attr(not(test), no_std)]
#![allow(unexpected_cfgs)]

// Re-exports
pub mod program_error {
    pub use jiminy_sysvar::program_error::*;
}
use program_error::*;

pub mod sysvar {
    pub use jiminy_sysvar::*;
}
use sysvar::*;

pub const ID_STR: &str = "SysvarRent111111111111111111111111111111111";

pub const ID: [u8; 32] = const_crypto::bs58::decode_pubkey(ID_STR);

/// Default rental rate in lamports/byte-year.
///
/// This calculation is based on:
/// - 10^9 lamports per SOL
/// - $1 per SOL
/// - $0.01 per megabyte day
/// - $3.65 per megabyte year
pub const DEFAULT_LAMPORTS_PER_BYTE_YEAR: u64 = 1_000_000_000 / 100 * 365 / (1024 * 1024);

/// Default amount of time (in years) the balance has to include rent for the
/// account to be rent exempt.
pub const DEFAULT_EXEMPTION_THRESHOLD: f64 = 2.0;

/// Default amount of time (in years) the balance has to include rent for the
/// account to be rent exempt as a `u64`.
///
/// This is used to avoid floating point operations for the default rent instance
const DEFAULT_EXEMPTION_THRESHOLD_AS_U64: u64 = 2;

/// The bitwise representation of the default exemption threshold.
const F64_DEFAULT_EXEMPTION_THRESHOLD_BITS: u64 = 4611686018427387904;

/// Default percentage of collected rent that is burned.
///
/// Valid values are in the range [0, 100]. The remaining percentage is
/// distributed to validators.
pub const DEFAULT_BURN_PERCENT: u8 = 50;

/// Account storage overhead for calculation of base rent.
///
/// This is the number of bytes required to store an account with no data. It is
/// added to an accounts data length when calculating [`Rent::min_balance`].
pub const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Rent {
    /// Rental rate in lamports per byte-year
    pub lamports_per_byte_year: u64,

    /// Exemption threshold in years.
    ///
    /// I CANNOT BELIEVE THEY ADDED A FLOAT TO A SYSVAR
    /// IN A VM THAT DOESNT SUPPORT IT BY DEFAULT
    pub exemption_threshold: f64,

    /// Burn percentage
    pub burn_percent: u8,
}

impl SysvarId for Rent {
    const ID: [u8; 32] = ID;
}

const _ASSERT_STRUCT_LEN: () = assert!(core::mem::size_of::<Rent>() == 24);
const _ASSERT_ACCOUNT_ALIGN: () = assert!(core::mem::align_of::<Rent>() == 8);
const _ASSERT_NO_INTERNAL_PADDING: () = {
    use core::mem::offset_of;

    assert!(offset_of!(Rent, lamports_per_byte_year) == 0);
    assert!(offset_of!(Rent, exemption_threshold) == 8);
    assert!(offset_of!(Rent, burn_percent) == 16);
};

unsafe impl SimpleSysvar for Rent {
    const ACCOUNT_LEN: usize = 17;
}

impl Rent {
    inherent_simple_sysvar_get!();
}

impl Rent {
    pub const DEFAULT: Self = Self {
        lamports_per_byte_year: DEFAULT_LAMPORTS_PER_BYTE_YEAR,
        exemption_threshold: DEFAULT_EXEMPTION_THRESHOLD,
        burn_percent: DEFAULT_BURN_PERCENT,
    };
}

impl Default for Rent {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl_cast_to_account_data!(Rent);

/// Deserialization from account data.
/// Cannot `impl_cast_from_account_data` due to the presence of external (suffix) padding bytes
impl Rent {
    #[inline]
    pub fn from_account_data(account_data: &[u8]) -> Result<Self, ProgramError> {
        match account_data.len() {
            Self::ACCOUNT_LEN => unsafe { Ok(Self::from_account_data_unchecked(account_data)) },
            _ => Err(ProgramError::from_builtin(
                BuiltInProgramError::InvalidAccountData,
            )),
        }
    }

    /// # Safety
    /// - account_data must be of `Self::ACCOUNT_LEN` length
    #[inline]
    pub unsafe fn from_account_data_unchecked(account_data: &[u8]) -> Self {
        Self::from_account_data_arr(&*account_data.as_ptr().cast())
    }

    // f64.from_le_bytes not yet stable in const in rustc 1.79
    #[inline]
    pub fn from_account_data_arr(account_data_arr: &[u8; Self::ACCOUNT_LEN]) -> Self {
        let Some((lamports_per_byte_year, rem)) = account_data_arr.split_first_chunk::<8>() else {
            unreachable!()
        };
        let Some((exemption_threshold, rem)) = rem.split_first_chunk::<8>() else {
            unreachable!()
        };
        let Some(burn_percent) = rem.first() else {
            unreachable!()
        };
        Self {
            lamports_per_byte_year: u64::from_le_bytes(*lamports_per_byte_year),
            exemption_threshold: f64::from_le_bytes(*exemption_threshold),
            burn_percent: *burn_percent,
        }
    }
}

impl Rent {
    // f64.to_le_bytes not yet stable in const in rustc 1.79
    /// Calculates the minimum balance for rent exemption.
    #[inline]
    pub fn min_balance(&self, data_len: usize) -> u64 {
        self.min_balance_u64(data_len as u64)
    }

    // f64.to_le_bytes not yet stable in const in rustc 1.79
    /// [`Self::min_balance`], but for `u64` `data_len`s instead of `usize`
    #[inline]
    pub fn min_balance_u64(&self, data_len: u64) -> u64 {
        // NB: this looks like overflow paradise but this is what the agave
        // implementation is like
        if self.is_default_rent_threshold() {
            ((ACCOUNT_STORAGE_OVERHEAD + data_len) * self.lamports_per_byte_year)
                * DEFAULT_EXEMPTION_THRESHOLD_AS_U64
        } else {
            (((ACCOUNT_STORAGE_OVERHEAD + data_len) * self.lamports_per_byte_year) as f64
                * self.exemption_threshold) as u64
        }
    }

    #[inline]
    fn is_default_rent_threshold(&self) -> bool {
        self.exemption_threshold.to_bits() == F64_DEFAULT_EXEMPTION_THRESHOLD_BITS
    }

    // all other methods from upstream are either test functions
    // or has to do with collecting rent from non-rent-exempt accounts,
    // which has pretty much been deprecated since
    // all accounts are required to be rent-exempt now (?)
}

#[cfg(test)]
mod tests {
    use proptest::{prop_assert_eq, prop_compose, proptest};
    use solana_rent::Rent as SolanaRent;

    use super::*;

    #[test]
    fn check_default_eq_solana() {
        let sr = SolanaRent::default();
        let r = Rent::default();

        assert_eq!(r.lamports_per_byte_year, sr.lamports_per_byte_year);
        assert_eq!(r.exemption_threshold, sr.exemption_threshold);
        assert_eq!(r.burn_percent, sr.burn_percent);
    }

    prop_compose! {
        // bounds because solana's impl uses unchecked arithmetic, which results in overflow
        // for large params
        fn rand_rent_params()
            (
                lamports_per_byte_year in 0u64..=u32::MAX as u64,
                exemption_threshold in 0.0..=255.0,
                burn_percent in 0u8..=100,
            )
            -> (u64, f64, u8) {

                (lamports_per_byte_year, exemption_threshold, burn_percent)
            }
    }

    proptest! {
        #[test]
        fn check_against_solana(
            (lamports_per_byte_year, exemption_threshold, burn_percent) in rand_rent_params(),
            data_len in 0..=u32::MAX as usize
        ) {
            let sr = SolanaRent { lamports_per_byte_year, exemption_threshold, burn_percent };
            let r = Rent{ lamports_per_byte_year, exemption_threshold, burn_percent };

            let sr_ser = bincode::serialize(&sr).unwrap();
            prop_assert_eq!(sr_ser.as_slice(), r.as_account_data_arr());
            prop_assert_eq!(sr.minimum_balance(data_len), r.min_balance(data_len));
        }
    }

    proptest! {
        #[test]
        fn serde_roundtrip(
            (lamports_per_byte_year, exemption_threshold, burn_percent) in rand_rent_params(),
        ) {
            let r = Rent { lamports_per_byte_year, exemption_threshold, burn_percent };
            let ser = r.as_account_data_arr();
            let de = Rent::from_account_data(ser).unwrap();
            prop_assert_eq!(de, r);
        }
    }
}
