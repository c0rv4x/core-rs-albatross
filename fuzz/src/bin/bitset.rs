use nimiq_collections::BitSet;
use nimiq_serde::Deserialize as _;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let _ = BitSet::deserialize_from_vec(data);
    })
}
