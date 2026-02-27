use chrono::Utc;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct TxRecord {
    pub id: u64,
    pub timestamp: String,
    pub category: String,
    pub action: String,
    pub sender: String,
    pub details: serde_json::Value,
    pub success: bool,
}

pub struct History {
    pub records: Vec<TxRecord>,
    counter: u64,
}

impl History {
    pub fn new() -> Self {
        History {
            records: Vec::new(),
            counter: 0,
        }
    }

    pub fn record(
        &mut self,
        action: &str,
        sender: &str,
        details: serde_json::Value,
        success: bool,
    ) -> u64 {
        self.counter += 1;
        let category = match action {
            a if a.starts_with("l1_") => "L1",
            a if a.starts_with("l2_") => "L2",
            a if a.starts_with("contract_") => "Contract",
            a if a.starts_with("wallet_") => "Wallet",
            _ => "System",
        }
        .to_string();

        self.records.push(TxRecord {
            id: self.counter,
            timestamp: Utc::now().to_rfc3339(),
            category,
            action: action.to_string(),
            sender: sender.to_string(),
            details,
            success,
        });
        self.counter
    }

    pub fn recent(&self, limit: usize) -> Vec<&TxRecord> {
        self.records.iter().rev().take(limit).collect()
    }

    pub fn count(&self) -> u64 {
        self.counter
    }
}

// ── API ─────────────────────────────────────────────────────────

use axum::{extract::State, Json};
use serde::Deserialize;
use crate::api::SharedState;

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct HistoryRes {
    pub total: u64,
    pub records: Vec<TxRecord>,
}

pub async fn get_history(
    State(state): State<SharedState>,
    axum::extract::Query(q): axum::extract::Query<HistoryQuery>,
) -> Json<HistoryRes> {
    let h = state.history.lock().await;
    let limit = q.limit.unwrap_or(50);
    let records: Vec<TxRecord> = h.recent(limit).into_iter().cloned().collect();
    Json(HistoryRes {
        total: h.count(),
        records,
    })
}

#[derive(Serialize)]
pub struct StatsRes {
    pub l1_blocks: usize,
    pub l1_valid: bool,
    pub l2_accounts: usize,
    pub l2_total_value: u64,
    pub l2_pending_txs: usize,
    pub l2_rollups: usize,
    pub contracts: usize,
    pub wallets: usize,
    pub total_transactions: u64,
}

pub async fn get_stats(State(state): State<SharedState>) -> Json<StatsRes> {
    let bc = state.blockchain.lock().await;
    let l2 = state.l2.lock().await;
    let contracts = state.contracts.lock().await;
    let wallets = state.wallets.lock().await;
    let history = state.history.lock().await;

    Json(StatsRes {
        l1_blocks: bc.chain.len(),
        l1_valid: bc.is_chain_valid(),
        l2_accounts: l2.state.balances.len(),
        l2_total_value: l2.state.balances.values().sum(),
        l2_pending_txs: l2.pending_txs.len(),
        l2_rollups: l2.rollups.len(),
        contracts: contracts.contracts.len(),
        wallets: wallets.wallets.len(),
        total_transactions: history.count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_retrieve() {
        let mut h = History::new();
        h.record("l1_mine", "alice", serde_json::json!({}), true);
        h.record("l2_deposit", "bob", serde_json::json!({"amount": 100}), true);
        assert_eq!(h.count(), 2);
        let recent = h.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].action, "l2_deposit");
        assert_eq!(recent[0].category, "L2");
    }

    #[test]
    fn recent_limit() {
        let mut h = History::new();
        for i in 0..20 {
            h.record("l1_mine", "a", serde_json::json!({"i": i}), true);
        }
        assert_eq!(h.recent(5).len(), 5);
    }
}
