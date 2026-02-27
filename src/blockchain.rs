use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: String,
    pub proof: u64,
    pub previous_hash: String,
    pub hash: String,
    pub data: String,
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut bc = Blockchain {
            chain: Vec::new(),
            difficulty: 4,
        };
        bc.create_block(1, "0".into(), "Genesis Block".into());
        bc
    }

    pub fn create_block(&mut self, proof: u64, previous_hash: String, data: String) -> Block {
        let index = self.chain.len() as u64 + 1;
        let timestamp = Utc::now().to_rfc3339();
        let hash = Self::calculate_block_hash(index, &timestamp, proof, &previous_hash, &data);

        let block = Block {
            index,
            timestamp,
            proof,
            previous_hash,
            hash,
            data,
        };

        self.chain.push(block.clone());
        block
    }

    pub fn get_previous_block(&self) -> &Block {
        self.chain.last().expect("chain is empty")
    }

    pub fn proof_of_work(&self, previous_proof: u64) -> u64 {
        let target = "0".repeat(self.difficulty);
        let mut new_proof: u64 = 1;
        loop {
            let hash = Self::hash_operation(new_proof, previous_proof);
            if hash.starts_with(&target) {
                return new_proof;
            }
            new_proof += 1;
        }
    }

    fn hash_operation(new_proof: u64, previous_proof: u64) -> String {
        let np = new_proof as i128;
        let pp = previous_proof as i128;
        let input = format!("{}", np * np - pp * pp);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn calculate_block_hash(
        index: u64,
        timestamp: &str,
        proof: u64,
        previous_hash: &str,
        data: &str,
    ) -> String {
        let input = format!("{}{}{}{}{}", index, timestamp, proof, previous_hash, data);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn is_chain_valid(&self) -> bool {
        if self.chain.len() <= 1 {
            return true;
        }

        let target = "0".repeat(self.difficulty);

        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];

            if current.previous_hash != previous.hash {
                return false;
            }

            let hash = Self::hash_operation(current.proof, previous.proof);
            if !hash.starts_with(&target) {
                return false;
            }

            let expected_hash = Self::calculate_block_hash(
                current.index,
                &current.timestamp,
                current.proof,
                &current.previous_hash,
                &current.data,
            );
            if current.hash != expected_hash {
                return false;
            }
        }

        true
    }

    pub fn mine_block(&mut self, data: String) -> Block {
        let prev = self.get_previous_block().clone();
        let proof = self.proof_of_work(prev.proof);
        self.create_block(proof, prev.hash, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_block_created() {
        let bc = Blockchain::new();
        assert_eq!(bc.chain.len(), 1);
        assert_eq!(bc.chain[0].index, 1);
        assert_eq!(bc.chain[0].previous_hash, "0");
        assert_eq!(bc.chain[0].data, "Genesis Block");
    }

    #[test]
    fn mine_adds_block() {
        let mut bc = Blockchain::new();
        bc.mine_block("Test data".into());
        assert_eq!(bc.chain.len(), 2);
        assert_eq!(bc.chain[1].data, "Test data");
        assert_eq!(bc.chain[1].previous_hash, bc.chain[0].hash);
    }

    #[test]
    fn chain_is_valid_after_mining() {
        let mut bc = Blockchain::new();
        bc.mine_block("Block 2".into());
        bc.mine_block("Block 3".into());
        assert!(bc.is_chain_valid());
    }

    #[test]
    fn tampered_chain_is_invalid() {
        let mut bc = Blockchain::new();
        bc.mine_block("Block 2".into());
        bc.chain[1].data = "tampered".into();
        assert!(!bc.is_chain_valid());
    }

    #[test]
    fn proof_of_work_produces_valid_hash() {
        let bc = Blockchain::new();
        let proof = bc.proof_of_work(1);
        let hash = Blockchain::hash_operation(proof, 1);
        assert!(hash.starts_with("0000"));
    }
}
