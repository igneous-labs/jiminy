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

pub const ID_STR: &str = "SysvarC1ock11111111111111111111111111111111";

pub const ID: [u8; 32] = const_crypto::bs58::decode_pubkey(ID_STR);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Clock {
    pub slot: u64,
    pub epoch_start_timestamp: i64,
    pub epoch: u64,
    pub leader_schedule_epoch: u64,
    pub unix_timestamp: i64,
}

impl SysvarId for Clock {
    const ID: [u8; 32] = ID;
}

const _ASSERT_ACCOUNT_LEN: () = assert!(core::mem::size_of::<Clock>() == 40);
const _ASSERT_ACCOUNT_ALIGN: () = assert!(core::mem::align_of::<Clock>() == 8);

unsafe impl SimpleSysvar for Clock {}

impl Clock {
    inherent_simple_sysvar_get!();
}

impl_cast_to_account_data!(Clock);

impl_cast_from_account_data!(Clock);

#[cfg(test)]
mod tests {
    use proptest::{prop_assert_eq, prop_compose, proptest};
    use solana_clock::Clock as SolanaClock;

    use super::*;

    fn assert_clock_eq(c: &Clock, s: &SolanaClock) {
        assert_eq!(c.slot, s.slot);
        assert_eq!(c.epoch_start_timestamp, s.epoch_start_timestamp);
        assert_eq!(c.epoch, s.epoch);
        assert_eq!(c.leader_schedule_epoch, s.leader_schedule_epoch);
        assert_eq!(c.unix_timestamp, s.unix_timestamp);
    }

    #[test]
    fn check_default_eq_solana() {
        let s = SolanaClock::default();
        let c = Clock::default();
        assert_clock_eq(&c, &s);
    }

    prop_compose! {
        fn rand_clock_params()
            (
                slot in 0..=u64::MAX,
                epoch_start_timestamp in 0..=i64::MAX,
                epoch in 0..=u64::MAX,
                leader_schedule_epoch in 0..=u64::MAX,
                unix_timestamp in 0..=i64::MAX,
            )
            -> (u64, i64, u64, u64, i64) {

                (slot, epoch_start_timestamp, epoch,  leader_schedule_epoch, unix_timestamp)
            }
    }

    proptest! {
        #[test]
        fn check_against_solana(
            (
                slot,
                epoch_start_timestamp,
                epoch,
                leader_schedule_epoch,
                unix_timestamp
            ) in rand_clock_params(),
        ) {
            let s = SolanaClock { slot, epoch_start_timestamp, epoch, leader_schedule_epoch, unix_timestamp };
            let c = Clock { slot, epoch_start_timestamp, epoch, leader_schedule_epoch, unix_timestamp };

            let s_ser = bincode::serialize(&s).unwrap();
            prop_assert_eq!(s_ser.as_slice(), c.as_account_data_arr());
        }
    }

    proptest! {
        #[test]
        fn serde_roundtrip(
            (
                slot,
                epoch_start_timestamp,
                epoch,
                leader_schedule_epoch,
                unix_timestamp
            ) in rand_clock_params(),
        ) {
            let c = Clock { slot, epoch_start_timestamp, epoch, leader_schedule_epoch, unix_timestamp };
            let ser = c.as_account_data_arr();
            // safety: aligned because underlying memory is c: Clock
            let de = unsafe { Clock::of_account_data(ser) }.unwrap();
            prop_assert_eq!(*de, c);
        }
    }
}
