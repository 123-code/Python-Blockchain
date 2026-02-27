use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::proof::ZkProof;
use super::state::AccountState;
use super::transaction::L2Transaction;
use crate::blockchain::Blockchain;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rollup {
    pub id: u64,
    pub proof: ZkProof,
    pub tx_count: usize,
    pub l1_block_index: u64,
    pub timestamp: String,
}

pub struct L2Engine {
    pub state: AccountState,
    committed_state: AccountState,
    pub pending_txs: Vec<L2Transaction>,
    pub rollups: Vec<Rollup>,
    rollup_counter: u64,
    pub batch_size: usize,
}

impl L2Engine {
    pub fn new(batch_size: usize) -> Self {
        L2Engine {
            state: AccountState::new(),
            committed_state: AccountState::new(),
            pending_txs: Vec::new(),
            rollups: Vec::new(),
            rollup_counter: 0,
            batch_size,
        }
    }

    pub fn submit_deposit(&mut self, account: &str, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err("amount must be > 0".into());
        }
        self.state.deposit(account, amount);
        self.pending_txs.push(L2Transaction::Deposit {
            account: account.to_string(),
            amount,
        });
        Ok(())
    }

    pub fn submit_transfer(
        &mut self,
        from: &str,
        to: &str,
        amount: u64,
    ) -> Result<(), String> {
        if amount == 0 {
            return Err("amount must be > 0".into());
        }
        if from == to {
            return Err("cannot transfer to self".into());
        }
        self.state.transfer(from, to, amount)?;
        self.pending_txs.push(L2Transaction::Transfer {
            from: from.to_string(),
            to: to.to_string(),
            amount,
        });
        Ok(())
    }

    pub fn submit_withdraw(&mut self, account: &str, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err("amount must be > 0".into());
        }
        self.state.withdraw(account, amount)?;
        self.pending_txs.push(L2Transaction::Withdraw {
            account: account.to_string(),
            amount,
        });
        Ok(())
    }

    /// Batches all pending transactions into a ZK-proved rollup and
    /// mines the proof onto the L1 chain.
    pub fn create_rollup(&mut self, blockchain: &mut Blockchain) -> Result<Rollup, String> {
        if self.pending_txs.is_empty() {
            return Err("no pending transactions to roll up".into());
        }

        let root_before = self.committed_state.state_root();
        let root_after = self.state.state_root();

        let proof = ZkProof::generate(&root_before, &root_after, &self.pending_txs);

        if !proof.verify() {
            return Err("generated proof failed verification (bug)".into());
        }

        let proof_json = serde_json::to_string(&proof).unwrap();
        let l1_data = format!("L2_ROLLUP:{}", proof_json);
        let l1_block = blockchain.mine_block(l1_data);

        self.rollup_counter += 1;
        let rollup = Rollup {
            id: self.rollup_counter,
            proof,
            tx_count: self.pending_txs.len(),
            l1_block_index: l1_block.index,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.rollups.push(rollup.clone());
        self.pending_txs.clear();
        self.committed_state = self.state.clone();

        Ok(rollup)
    }

    pub fn should_auto_rollup(&self) -> bool {
        self.pending_txs.len() >= self.batch_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_updates_balance() {
        let mut e = L2Engine::new(10);
        e.submit_deposit("alice", 100).unwrap();
        assert_eq!(e.state.balance("alice"), 100);
        assert_eq!(e.pending_txs.len(), 1);
    }

    #[test]
    fn transfer_validates_balance() {
        let mut e = L2Engine::new(10);
        e.submit_deposit("alice", 50).unwrap();
        assert!(e.submit_transfer("alice", "bob", 60).is_err());
        e.submit_transfer("alice", "bob", 30).unwrap();
        assert_eq!(e.state.balance("alice"), 20);
        assert_eq!(e.state.balance("bob"), 30);
    }

    #[test]
    fn rollup_clears_pending_and_produces_proof() {
        let mut e = L2Engine::new(10);
        let mut bc = Blockchain::new();
        e.submit_deposit("alice", 100).unwrap();
        e.submit_transfer("alice", "bob", 25).unwrap();
        let rollup = e.create_rollup(&mut bc).unwrap();
        assert!(rollup.proof.verify());
        assert_eq!(rollup.tx_count, 2);
        assert!(e.pending_txs.is_empty());
        assert_eq!(e.rollups.len(), 1);
        assert_eq!(bc.chain.len(), 2); // genesis + rollup block
    }

    #[test]
    fn rollup_empty_fails() {
        let mut e = L2Engine::new(10);
        let mut bc = Blockchain::new();
        assert!(e.create_rollup(&mut bc).is_err());
    }

    #[test]
    fn sequential_rollups() {
        let mut e = L2Engine::new(10);
        let mut bc = Blockchain::new();
        e.submit_deposit("alice", 100).unwrap();
        e.create_rollup(&mut bc).unwrap();
        e.submit_transfer("alice", "bob", 10).unwrap();
        let r2 = e.create_rollup(&mut bc).unwrap();
        assert!(r2.proof.verify());
        assert_eq!(r2.proof.state_root_after, e.committed_state.state_root());
        assert_eq!(e.rollups.len(), 2);
    }

    #[test]
    fn zero_amount_rejected() {
        let mut e = L2Engine::new(10);
        assert!(e.submit_deposit("a", 0).is_err());
        assert!(e.submit_transfer("a", "b", 0).is_err());
        assert!(e.submit_withdraw("a", 0).is_err());
    }
}
