use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::compiler;
use super::executor::{self, ExecContext, ExecResult, DEFAULT_GAS_LIMIT};
use super::opcode::Op;
use crate::l2::merkle;
use crate::l2::state::AccountState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contract {
    pub address: String,
    pub source: String,
    #[serde(skip)]
    pub code: Vec<Op>,
    pub storage: BTreeMap<String, i64>,
    pub balance: u64,
    pub deployer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractInfo {
    pub address: String,
    pub deployer: String,
    pub balance: u64,
    pub storage: BTreeMap<String, i64>,
    pub source: String,
}

pub struct ContractStore {
    pub contracts: BTreeMap<String, Contract>,
    deploy_nonce: u64,
}

impl ContractStore {
    pub fn new() -> Self {
        ContractStore {
            contracts: BTreeMap::new(),
            deploy_nonce: 0,
        }
    }

    /// Deploy a contract. If `init_args` are provided, the code is executed
    /// once as a constructor with those args. Otherwise the code is stored
    /// without execution (call it separately to initialize).
    pub fn deploy(
        &mut self,
        sender: &str,
        source: &str,
        init_args: Option<Vec<i64>>,
        accounts: &mut AccountState,
        block_height: u64,
    ) -> Result<(String, Option<ExecResult>), String> {
        let code = compiler::compile(source)?;
        if code.is_empty() {
            return Err("empty contract code".into());
        }

        self.deploy_nonce += 1;
        let address = generate_address(sender, self.deploy_nonce);

        let mut contract = Contract {
            address: address.clone(),
            source: source.to_string(),
            code,
            storage: BTreeMap::new(),
            balance: 0,
            deployer: sender.to_string(),
        };

        let exec_result = if let Some(args) = init_args {
            let ctx = ExecContext {
                caller: sender.into(),
                value: 0,
                args,
                block_height,
                contract_balance: 0,
            };
            let result = executor::execute(
                &contract.code,
                &ctx,
                &mut contract.storage,
                DEFAULT_GAS_LIMIT,
            );
            if !result.success {
                return Err(format!(
                    "constructor failed: {}",
                    result.error.unwrap_or_default()
                ));
            }
            for change in &result.balance_changes {
                accounts.deposit(&change.account, change.amount);
            }
            Some(result)
        } else {
            None
        };

        self.contracts.insert(address.clone(), contract);
        Ok((address, exec_result))
    }

    pub fn call(
        &mut self,
        address: &str,
        sender: &str,
        value: u64,
        args: Vec<i64>,
        accounts: &mut AccountState,
        block_height: u64,
        gas_limit: u64,
    ) -> Result<ExecResult, String> {
        let contract = self
            .contracts
            .get_mut(address)
            .ok_or_else(|| format!("contract not found: {}", address))?;

        // Transfer value from sender to contract
        if value > 0 {
            let bal = accounts.balance(sender);
            if bal < value {
                return Err(format!(
                    "insufficient balance: {} has {}, needs {}",
                    sender, bal, value
                ));
            }
            accounts.withdraw(sender, value)?;
        }

        let code = contract.code.clone();
        let ctx = ExecContext {
            caller: sender.into(),
            value,
            args,
            block_height,
            contract_balance: contract.balance,
        };

        let result = executor::execute(&code, &ctx, &mut contract.storage, gas_limit);

        if result.success {
            // Update contract balance: add value, subtract transfers
            contract.balance += value;
            let total_transferred: u64 =
                result.balance_changes.iter().map(|c| c.amount).sum();
            contract.balance -= total_transferred;

            // Credit transfer recipients
            for change in &result.balance_changes {
                accounts.deposit(&change.account, change.amount);
            }
        } else {
            // Revert: return value to sender
            if value > 0 {
                accounts.deposit(sender, value);
            }
        }

        Ok(result)
    }

    pub fn get(&self, address: &str) -> Option<ContractInfo> {
        self.contracts.get(address).map(|c| ContractInfo {
            address: c.address.clone(),
            deployer: c.deployer.clone(),
            balance: c.balance,
            storage: c.storage.clone(),
            source: c.source.clone(),
        })
    }

    pub fn get_storage_value(&self, address: &str, key: &str) -> Option<i64> {
        self.contracts
            .get(address)
            .and_then(|c| c.storage.get(key).copied())
    }

    /// Hash of all contract states for inclusion in the L2 state root.
    pub fn state_hash(&self) -> String {
        let leaves: Vec<String> = self
            .contracts
            .iter()
            .map(|(addr, c)| {
                let storage_str: String = c
                    .storage
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(",");
                merkle::hash_data(
                    format!("{}:{}:{}", addr, c.balance, storage_str).as_bytes(),
                )
            })
            .collect();
        if leaves.is_empty() {
            return merkle::hash_data(b"no_contracts");
        }
        crate::l2::merkle::MerkleTree::from_leaves(&leaves).root
    }
}

fn generate_address(deployer: &str, nonce: u64) -> String {
    let mut h = Sha256::new();
    h.update(format!("contract:{}:{}", deployer, nonce).as_bytes());
    let hash = hex::encode(h.finalize());
    format!("0x{}", &hash[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_with_init() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        let (addr, result) = store
            .deploy(
                "alice",
                "PUSH 42\nSTORE answer\nHALT",
                Some(vec![]),
                &mut accounts,
                1,
            )
            .unwrap();
        assert!(addr.starts_with("0x"));
        assert!(result.unwrap().success);
        assert_eq!(store.get_storage_value(&addr, "answer"), Some(42));
    }

    #[test]
    fn deploy_without_init() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        let (addr, result) = store
            .deploy("alice", "LOAD x\nHALT", None, &mut accounts, 1)
            .unwrap();
        assert!(result.is_none());
        assert!(store.get(&addr).is_some());
    }

    #[test]
    fn deploy_bad_code() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        assert!(store.deploy("a", "INVALID", None, &mut accounts, 1).is_err());
    }

    #[test]
    fn call_contract() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        let code = "\
DUP
PUSH 1
EQ
JUMPIF :inc
REVERT
:inc
POP
LOAD counter
PUSH 1
ADD
STORE counter
LOAD counter
HALT";
        let (addr, _) = store.deploy("alice", code, None, &mut accounts, 1).unwrap();
        let r = store
            .call(&addr, "bob", 0, vec![1], &mut accounts, 2, DEFAULT_GAS_LIMIT)
            .unwrap();
        assert!(r.success);
        assert_eq!(r.return_value, Some(1));

        let r2 = store
            .call(&addr, "bob", 0, vec![1], &mut accounts, 3, DEFAULT_GAS_LIMIT)
            .unwrap();
        assert_eq!(r2.return_value, Some(2));
    }

    #[test]
    fn call_with_value() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        accounts.deposit("alice", 1000);
        let (addr, _) = store
            .deploy("alice", "CALLVALUE\nHALT", None, &mut accounts, 1)
            .unwrap();
        let r = store
            .call(&addr, "alice", 100, vec![], &mut accounts, 2, DEFAULT_GAS_LIMIT)
            .unwrap();
        assert!(r.success);
        assert_eq!(r.return_value, Some(100));
        assert_eq!(accounts.balance("alice"), 900);
        assert_eq!(store.contracts.get(&addr).unwrap().balance, 100);
    }

    #[test]
    fn call_revert_returns_value() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        accounts.deposit("alice", 500);
        let (addr, _) = store
            .deploy("alice", "REVERT", None, &mut accounts, 1)
            .unwrap();
        let r = store
            .call(&addr, "alice", 100, vec![], &mut accounts, 2, DEFAULT_GAS_LIMIT)
            .unwrap();
        assert!(!r.success);
        assert_eq!(accounts.balance("alice"), 500); // value returned
    }

    #[test]
    fn transfer_from_contract() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        accounts.deposit("alice", 500);

        let code = "PUSH 50\nTRANSFER bob\nHALT";
        let (addr, _) = store.deploy("alice", code, None, &mut accounts, 1).unwrap();

        // Fund the contract by calling with value
        let r = store
            .call(&addr, "alice", 200, vec![], &mut accounts, 2, DEFAULT_GAS_LIMIT)
            .unwrap();
        assert!(r.success);
        assert_eq!(accounts.balance("bob"), 50);
        assert_eq!(store.contracts.get(&addr).unwrap().balance, 150);
    }

    #[test]
    fn unique_addresses() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        let (a1, _) = store.deploy("alice", "HALT", None, &mut accounts, 1).unwrap();
        let (a2, _) = store.deploy("alice", "HALT", None, &mut accounts, 1).unwrap();
        assert_ne!(a1, a2);
    }

    #[test]
    fn state_hash_changes() {
        let mut store = ContractStore::new();
        let mut accounts = AccountState::new();
        let h1 = store.state_hash();
        store
            .deploy("alice", "PUSH 1\nSTORE x\nHALT", Some(vec![]), &mut accounts, 1)
            .unwrap();
        let h2 = store.state_hash();
        assert_ne!(h1, h2);
    }
}
