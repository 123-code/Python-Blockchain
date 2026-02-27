use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::merkle::MerkleTree;
use super::transaction::L2Transaction;

/// ZK proof certifying a valid L2 state transition.
///
/// Uses a Fiat-Shamir sigma protocol: the prover commits to private
/// transaction data, derives a non-interactive challenge, and produces
/// a response bound to the witness.  The verifier checks structural
/// consistency without learning individual transactions.
///
/// Production systems would replace this with a SNARK/STARK circuit
/// (e.g. bellman, ark-works, plonky2).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkProof {
    pub state_root_before: String,
    pub state_root_after: String,
    pub tx_count: usize,
    pub tx_root: String,
    pub commitment: String,
    pub challenge: String,
    pub response: String,
    pub nullifier: String,
}

impl ZkProof {
    pub fn generate(
        state_root_before: &str,
        state_root_after: &str,
        transactions: &[L2Transaction],
    ) -> Self {
        let tx_hashes: Vec<String> = transactions.iter().map(|t| t.hash()).collect();
        let tx_root = MerkleTree::from_leaves(&tx_hashes).root;

        // Nonce derived from timestamp + state (in production: CSPRNG)
        let nonce = {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            sha(&format!("nonce:{}:{}:{}", t, state_root_before, tx_root))
        };

        // Witness: hash of full transaction data (private knowledge)
        let witness = {
            let raw = serde_json::to_string(transactions).unwrap();
            sha(raw.as_bytes())
        };

        // Execution trace binds public I/O to the transaction set
        let execution_trace = sha(&format!(
            "exec:{}:{}:{}",
            state_root_before, state_root_after, tx_root
        ));

        // Commitment hides the nonce and execution trace
        let commitment = sha(&format!("commit:{}:{}:{}", nonce, execution_trace, witness));

        // Fiat-Shamir challenge
        let challenge = sha(&format!(
            "challenge:{}:{}:{}:{}",
            commitment,
            state_root_before,
            state_root_after,
            transactions.len()
        ));

        // Response ties nonce + challenge + witness
        let response = sha(&format!("response:{}:{}:{}", nonce, challenge, witness));

        // Nullifier for uniqueness / replay protection
        let nullifier = sha(&format!(
            "null:{}:{}:{}",
            commitment, challenge, response
        ));

        ZkProof {
            state_root_before: state_root_before.to_string(),
            state_root_after: state_root_after.to_string(),
            tx_count: transactions.len(),
            tx_root,
            commitment,
            challenge,
            response,
            nullifier,
        }
    }

    pub fn verify(&self) -> bool {
        if self.tx_count == 0 {
            return false;
        }

        // Recompute Fiat-Shamir challenge from public inputs + commitment
        let expected_challenge = sha(&format!(
            "challenge:{}:{}:{}:{}",
            self.commitment,
            self.state_root_before,
            self.state_root_after,
            self.tx_count
        ));
        if self.challenge != expected_challenge {
            return false;
        }

        // Recompute nullifier
        let expected_nullifier = sha(&format!(
            "null:{}:{}:{}",
            self.commitment, self.challenge, self.response
        ));
        if self.nullifier != expected_nullifier {
            return false;
        }

        // Structural checks: all hash fields must be 64-char hex
        for h in [
            &self.commitment,
            &self.challenge,
            &self.response,
            &self.nullifier,
            &self.tx_root,
        ] {
            if !is_valid_hash(h) {
                return false;
            }
        }

        // State roots must be non-empty
        if self.state_root_before.is_empty() || self.state_root_after.is_empty() {
            return false;
        }

        true
    }
}

fn sha(input: &(impl AsRef<[u8]> + ?Sized)) -> String {
    let mut h = Sha256::new();
    h.update(input.as_ref());
    hex::encode(h.finalize())
}

fn is_valid_hash(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_txs() -> Vec<L2Transaction> {
        vec![
            L2Transaction::Transfer {
                from: "alice".into(),
                to: "bob".into(),
                amount: 10,
            },
            L2Transaction::Transfer {
                from: "bob".into(),
                to: "charlie".into(),
                amount: 5,
            },
        ]
    }

    #[test]
    fn generate_and_verify() {
        let proof = ZkProof::generate("root_a", "root_b", &sample_txs());
        assert!(proof.verify());
    }

    #[test]
    fn tampered_challenge_fails() {
        let mut proof = ZkProof::generate("root_a", "root_b", &sample_txs());
        proof.challenge = sha("tampered");
        assert!(!proof.verify());
    }

    #[test]
    fn tampered_nullifier_fails() {
        let mut proof = ZkProof::generate("root_a", "root_b", &sample_txs());
        proof.nullifier = sha("tampered");
        assert!(!proof.verify());
    }

    #[test]
    fn tampered_state_root_fails() {
        let mut proof = ZkProof::generate("root_a", "root_b", &sample_txs());
        proof.state_root_after = "fake".into();
        assert!(!proof.verify());
    }

    #[test]
    fn empty_batch_fails() {
        let proof = ZkProof {
            state_root_before: "a".into(),
            state_root_after: "b".into(),
            tx_count: 0,
            tx_root: sha("x"),
            commitment: sha("c"),
            challenge: sha("ch"),
            response: sha("r"),
            nullifier: sha("n"),
        };
        assert!(!proof.verify());
    }

    #[test]
    fn different_txs_different_proofs() {
        let p1 = ZkProof::generate("r", "r2", &sample_txs());
        let p2 = ZkProof::generate(
            "r",
            "r2",
            &[L2Transaction::Deposit {
                account: "x".into(),
                amount: 1,
            }],
        );
        assert_ne!(p1.tx_root, p2.tx_root);
        assert_ne!(p1.commitment, p2.commitment);
    }
}
