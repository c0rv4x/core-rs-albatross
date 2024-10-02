use nimiq_primitives::trie::trie_node::TrieNode;
use nimiq_serde::Deserialize as _;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let _ = TrieNode::deserialize_from_vec(data);
    })
}
