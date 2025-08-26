#![allow(unexpected_cfgs)]
#![cfg(not(target_os = "solana"))]

use std::{fs::File, io::Write, os::unix::fs::MetadataExt, path::PathBuf};

use proptest::{
    prelude::{Just, Strategy},
    strategy::Union,
};

pub fn silence_mollusk_prog_logs() {
    solana_logger::setup_with_default(
        "solana_rbpf::vm=warn,\
         solana_runtime::message_processor=warn,\
         solana_runtime::system_instruction_processor=warn",
    );
}

fn range_excl(excl: u128) -> impl Strategy<Value = u128> + Clone {
    let lower = (excl > 0).then_some(0..=excl - 1);
    let higher = (excl < u128::MAX).then_some(excl + 1..=u128::MAX);
    Union::new(lower.into_iter().chain(higher))
}

fn merge_pk([l, h]: [u128; 2]) -> [u8; 32] {
    let mut pk1 = [0u8; 32];
    pk1[..16].copy_from_slice(&l.to_le_bytes());
    pk1[16..].copy_from_slice(&h.to_le_bytes());
    pk1
}

pub fn two_different_pubkeys() -> impl Strategy<Value = [[u8; 32]; 2]> {
    (0..=u128::MAX, 0..=u128::MAX).prop_flat_map(|(l1, h1)| {
        let [l2, h2] = [l1, h1].map(range_excl);
        [
            Just(merge_pk([l1, h1])).boxed(),
            Union::new([
                [l2.clone(), h2.clone()].prop_map(merge_pk).boxed(),
                [Just(l1).boxed(), h2.boxed()].prop_map(merge_pk).boxed(),
                [l2.boxed(), Just(h1).boxed()].prop_map(merge_pk).boxed(),
            ])
            .boxed(),
        ]
    })
}

const BENCH_RES_DIR: &str = "bench-res";

pub fn save_cus_to_file(name: &str, compute_units_consumed: u64) {
    let mut f = File::create(
        PathBuf::from(BENCH_RES_DIR)
            .join(name)
            .with_extension("cus.txt"),
    )
    .unwrap();
    f.write_all(compute_units_consumed.to_string().as_bytes())
        .unwrap();
}

pub fn save_binsize_to_file(prog_name: &str) {
    let size = File::open(
        PathBuf::from(std::env::var("SBF_OUT_DIR").unwrap())
            .join(prog_name)
            .with_extension("so"),
    )
    .unwrap()
    .metadata()
    .unwrap()
    .size();
    File::create(
        PathBuf::from(BENCH_RES_DIR)
            .join("binsize")
            .with_extension("txt"),
    )
    .unwrap()
    .write_all(size.to_string().as_bytes())
    .unwrap();
}
