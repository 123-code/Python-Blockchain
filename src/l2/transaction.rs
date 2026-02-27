use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum L2Transaction {
    Deposit {
        account: String,
        amount: u64,
    },
    Transfer {
        from: String,
        to: String,
        amount: u64,
    },
    Withdraw {
        account: String,
        amount: u64,
    },
}

impl L2Transaction {
    pub fn hash(&self) -> String {
        let data = serde_json::to_string(self).unwrap();
        let mut h = Sha256::new();
        h.update(data.as_bytes());
        hex::encode(h.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let a = L2Transaction::Transfer {
            from: "alice".into(),
            to: "bob".into(),
            amount: 10,
        };
        let b = L2Transaction::Transfer {
            from: "alice".into(),
            to: "bob".into(),
            amount: 10,
        };
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn different_txs_different_hash() {
        let a = L2Transaction::Deposit {
            account: "alice".into(),
            amount: 10,
        };
        let b = L2Transaction::Deposit {
            account: "alice".into(),
            amount: 20,
        };
        assert_ne!(a.hash(), b.hash());
    }
}
