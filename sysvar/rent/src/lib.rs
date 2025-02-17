#![cfg_attr(not(test), no_std)]

// Re-exports
pub mod program_error {
    pub use jiminy_program_error::*;
}

use program_error::*;

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
/// added to an accounts data length when calculating [`Rent::minimum_balance`].
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

impl Rent {
    pub const ACCOUNT_SIZE: usize = 17;

    pub const DEFAULT: Self = Self {
        lamports_per_byte_year: DEFAULT_LAMPORTS_PER_BYTE_YEAR,
        exemption_threshold: DEFAULT_EXEMPTION_THRESHOLD,
        burn_percent: DEFAULT_BURN_PERCENT,
    };
}

// onchain sysvar
impl Rent {
    /// [`jiminy_syscall::sol_get_rent_sysvar`]
    #[inline]
    pub fn get() -> Result<Self, ProgramError> {
        #[cfg(target_os = "solana")]
        {
            use core::mem::MaybeUninit;

            // NB: the only reason why the pointer casting here works is because
            // or repr(C) and because
            // the fields of the struct have no padding in-between
            let mut ret: MaybeUninit<Self> = MaybeUninit::uninit();
            let res = unsafe { jiminy_syscall::sol_get_rent_sysvar(ret.as_mut_ptr().cast()) };
            match res {
                0 => Ok(unsafe { ret.assume_init() }),
                e => Err(e.into()),
            }
        }

        #[cfg(not(target_os = "solana"))]
        {
            unreachable!()
        }
    }
}

// serde
impl Rent {
    #[inline]
    pub fn from_account_data(account_data: &[u8]) -> Result<Self, ProgramError> {
        if account_data.len() != Self::ACCOUNT_SIZE {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(unsafe { Self::from_account_data_unchecked(account_data) })
        }
    }

    /// # Safety
    /// - account_data must be of [`Self::ACCOUNT_SIZE`] size
    #[inline]
    pub unsafe fn from_account_data_unchecked(account_data: &[u8]) -> Self {
        Self::from_account_data_arr(&*account_data.as_ptr().cast())
    }

    #[inline]
    pub fn from_account_data_arr(account_data_arr: &[u8; Self::ACCOUNT_SIZE]) -> Self {
        // safety: bounds-checked by type
        let lamports_per_byte_year =
            u64::from_le_bytes(unsafe { *account_data_arr.get_unchecked(0..8).as_ptr().cast() });
        let exemption_threshold =
            f64::from_le_bytes(unsafe { *account_data_arr.get_unchecked(8..16).as_ptr().cast() });
        let burn_percent = unsafe { *account_data_arr.get_unchecked(16) };
        Self {
            lamports_per_byte_year,
            exemption_threshold,
            burn_percent,
        }
    }

    #[inline]
    pub fn to_account_data(&self) -> [u8; Self::ACCOUNT_SIZE] {
        // TODO: determine whether its worth it using MaybeUninit here instead
        // of zero-initializing
        let mut res = [0u8; Self::ACCOUNT_SIZE];
        res[..8].copy_from_slice(&self.lamports_per_byte_year.to_le_bytes());
        res[8..16].copy_from_slice(&self.exemption_threshold.to_le_bytes());
        res[16] = self.burn_percent;
        res
    }
}

impl Rent {
    /// Calculates the minimum balance for rent exemption.
    #[inline]
    pub fn minimum_balance(&self, data_len: usize) -> u64 {
        self.minimum_balance_u64(data_len as u64)
    }

    /// [`Self::minimum_balance`], but for `u64` `data_len`s instead of `usize`
    #[inline]
    pub fn minimum_balance_u64(&self, data_len: u64) -> u64 {
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

    // TODO: convert to const fn once f64::to_bits() is stable
    #[inline]
    fn is_default_rent_threshold(&self) -> bool {
        self.exemption_threshold.to_bits() == F64_DEFAULT_EXEMPTION_THRESHOLD_BITS
    }

    // all other methods from upstream are either test functions
    // or has to do with collecting rent from non-rent-exempt accounts,
    // which has pretty much been deprecated since
    // all accounts are required to be rent-exempt now (?)
}

impl Default for Rent {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
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
            let r = Rent { lamports_per_byte_year, exemption_threshold, burn_percent };

            let sr_ser = bincode::serialize(&sr).unwrap();
            prop_assert_eq!(sr_ser.as_slice(), &r.to_account_data());
            prop_assert_eq!(sr.minimum_balance(data_len), r.minimum_balance(data_len));
        }
    }

    proptest! {
        #[test]
        fn serde_roundtrip(
            (lamports_per_byte_year, exemption_threshold, burn_percent) in rand_rent_params(),
        ) {
            let r = Rent { lamports_per_byte_year, exemption_threshold, burn_percent };
            let ser = r.to_account_data();
            let de = Rent::from_account_data_arr(&ser);
            prop_assert_eq!(de, r);
        }
    }
}
