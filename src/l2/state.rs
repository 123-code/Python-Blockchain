use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::merkle::MerkleTree;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountState {
    pub balances: BTreeMap<String, u64>,
}

impl AccountState {
    pub fn new() -> Self {
        AccountState {
            balances: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, account: &str, amount: u64) {
        *self.balances.entry(account.to_string()).or_insert(0) += amount;
    }

    pub fn transfer(&mut self, from: &str, to: &str, amount: u64) -> Result<(), String> {
        let bal = self.balance(from);
        if bal < amount {
            return Err(format!(
                "insufficient balance: {} has {}, needs {}",
                from, bal, amount
            ));
        }
        *self.balances.entry(from.to_string()).or_insert(0) -= amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, account: &str, amount: u64) -> Result<(), String> {
        let bal = self.balance(account);
        if bal < amount {
            return Err(format!(
                "insufficient balance: {} has {}, needs {}",
                account, bal, amount
            ));
        }
        *self.balances.entry(account.to_string()).or_insert(0) -= amount;
        Ok(())
    }

    pub fn balance(&self, account: &str) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn state_root(&self) -> String {
        let leaves: Vec<String> = self
            .balances
            .iter()
            .map(|(k, v)| {
                super::merkle::hash_data(format!("{}:{}", k, v).as_bytes())
            })
            .collect();
        if leaves.is_empty() {
            return MerkleTree::empty().root;
        }
        MerkleTree::from_leaves(&leaves).root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_and_balance() {
        let mut s = AccountState::new();
        s.deposit("alice", 100);
        assert_eq!(s.balance("alice"), 100);
        assert_eq!(s.balance("bob"), 0);
    }

    #[test]
    fn transfer_success() {
        let mut s = AccountState::new();
        s.deposit("alice", 100);
        s.transfer("alice", "bob", 40).unwrap();
        assert_eq!(s.balance("alice"), 60);
        assert_eq!(s.balance("bob"), 40);
    }

    #[test]
    fn transfer_insufficient() {
        let mut s = AccountState::new();
        s.deposit("alice", 10);
        assert!(s.transfer("alice", "bob", 50).is_err());
    }

    #[test]
    fn withdraw_success() {
        let mut s = AccountState::new();
        s.deposit("alice", 100);
        s.withdraw("alice", 30).unwrap();
        assert_eq!(s.balance("alice"), 70);
    }

    #[test]
    fn state_root_changes_on_mutation() {
        let mut s = AccountState::new();
        s.deposit("alice", 100);
        let r1 = s.state_root();
        s.deposit("alice", 1);
        let r2 = s.state_root();
        assert_ne!(r1, r2);
    }

    #[test]
    fn state_root_deterministic() {
        let mut a = AccountState::new();
        a.deposit("alice", 100);
        a.deposit("bob", 50);
        let mut b = AccountState::new();
        b.deposit("alice", 100);
        b.deposit("bob", 50);
        assert_eq!(a.state_root(), b.state_root());
    }
}
