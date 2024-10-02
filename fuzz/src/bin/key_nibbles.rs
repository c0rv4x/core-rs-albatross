use nimiq_primitives::key_nibbles::KeyNibbles;
use nimiq_serde::Deserialize as _;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let _ = KeyNibbles::deserialize_from_vec(data);
    })
}
