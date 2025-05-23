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
