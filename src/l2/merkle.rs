use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleTree {
    pub root: String,
    pub leaf_count: usize,
}

impl MerkleTree {
    pub fn from_leaves(leaves: &[String]) -> Self {
        let root = compute_root(leaves);
        MerkleTree {
            root,
            leaf_count: leaves.len(),
        }
    }

    pub fn empty() -> Self {
        MerkleTree {
            root: hash_data(b"empty_tree"),
            leaf_count: 0,
        }
    }
}

fn compute_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return hash_data(b"empty_tree");
    }
    if leaves.len() == 1 {
        return leaves[0].clone();
    }

    let mut level: Vec<String> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                next.push(hash_pair(&chunk[0], &chunk[1]));
            } else {
                next.push(hash_pair(&chunk[0], &chunk[0]));
            }
        }
        level = next;
    }
    level.into_iter().next().unwrap()
}

pub fn hash_pair(a: &str, b: &str) -> String {
    let mut h = Sha256::new();
    h.update(a.as_bytes());
    h.update(b.as_bytes());
    hex::encode(h.finalize())
}

pub fn hash_data(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree() {
        let t = MerkleTree::empty();
        assert_eq!(t.leaf_count, 0);
        assert_eq!(t.root.len(), 64);
    }

    #[test]
    fn single_leaf() {
        let t = MerkleTree::from_leaves(&["abc".into()]);
        assert_eq!(t.root, "abc");
        assert_eq!(t.leaf_count, 1);
    }

    #[test]
    fn two_leaves() {
        let t = MerkleTree::from_leaves(&["a".into(), "b".into()]);
        assert_eq!(t.root, hash_pair("a", "b"));
    }

    #[test]
    fn deterministic() {
        let a = MerkleTree::from_leaves(&["x".into(), "y".into(), "z".into()]);
        let b = MerkleTree::from_leaves(&["x".into(), "y".into(), "z".into()]);
        assert_eq!(a.root, b.root);
    }

    #[test]
    fn different_order_different_root() {
        let a = MerkleTree::from_leaves(&["x".into(), "y".into()]);
        let b = MerkleTree::from_leaves(&["y".into(), "x".into()]);
        assert_ne!(a.root, b.root);
    }
}
