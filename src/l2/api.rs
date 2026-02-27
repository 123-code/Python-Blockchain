use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::SharedState;
use super::engine::Rollup;
use super::proof::ZkProof;

// ── request bodies ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DepositReq {
    pub account: String,
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct TransferReq {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct WithdrawReq {
    pub account: String,
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct VerifyProofReq {
    pub proof: ZkProof,
}

// ── response bodies ─────────────────────────────────────────────

#[derive(Serialize)]
pub struct OkMsg {
    pub message: String,
}

#[derive(Serialize)]
pub struct BalanceRes {
    pub account: String,
    pub balance: u64,
}

#[derive(Serialize)]
pub struct BalancesRes {
    pub balances: std::collections::BTreeMap<String, u64>,
    pub state_root: String,
}

#[derive(Serialize)]
pub struct PendingRes {
    pub count: usize,
    pub transactions: Vec<super::transaction::L2Transaction>,
}

#[derive(Serialize)]
pub struct RollupRes {
    pub message: String,
    pub rollup: Rollup,
}

#[derive(Serialize)]
pub struct RollupsRes {
    pub count: usize,
    pub rollups: Vec<Rollup>,
}

#[derive(Serialize)]
pub struct VerifyRes {
    pub valid: bool,
}

// ── handlers ────────────────────────────────────────────────────

pub async fn deposit(
    State(state): State<SharedState>,
    Json(body): Json<DepositReq>,
) -> Result<(StatusCode, Json<OkMsg>), (StatusCode, Json<OkMsg>)> {
    let mut l2 = state.l2.lock().await;
    l2.submit_deposit(&body.account, body.amount)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(OkMsg { message: e })))?;

    let auto = l2.should_auto_rollup();
    if auto {
        let mut bc = state.blockchain.lock().await;
        let _ = l2.create_rollup(&mut bc);
    }

    Ok((
        StatusCode::CREATED,
        Json(OkMsg {
            message: format!(
                "deposited {} to {}{}",
                body.amount,
                body.account,
                if auto { " (auto-rollup triggered)" } else { "" }
            ),
        }),
    ))
}

pub async fn transfer(
    State(state): State<SharedState>,
    Json(body): Json<TransferReq>,
) -> Result<(StatusCode, Json<OkMsg>), (StatusCode, Json<OkMsg>)> {
    let mut l2 = state.l2.lock().await;
    l2.submit_transfer(&body.from, &body.to, body.amount)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(OkMsg { message: e })))?;

    let auto = l2.should_auto_rollup();
    if auto {
        let mut bc = state.blockchain.lock().await;
        let _ = l2.create_rollup(&mut bc);
    }

    Ok((
        StatusCode::CREATED,
        Json(OkMsg {
            message: format!(
                "transferred {} from {} to {}{}",
                body.amount,
                body.from,
                body.to,
                if auto { " (auto-rollup triggered)" } else { "" }
            ),
        }),
    ))
}

pub async fn withdraw(
    State(state): State<SharedState>,
    Json(body): Json<WithdrawReq>,
) -> Result<(StatusCode, Json<OkMsg>), (StatusCode, Json<OkMsg>)> {
    let mut l2 = state.l2.lock().await;
    l2.submit_withdraw(&body.account, body.amount)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(OkMsg { message: e })))?;

    let auto = l2.should_auto_rollup();
    if auto {
        let mut bc = state.blockchain.lock().await;
        let _ = l2.create_rollup(&mut bc);
    }

    Ok((
        StatusCode::CREATED,
        Json(OkMsg {
            message: format!(
                "withdrew {} from {}{}",
                body.amount,
                body.account,
                if auto { " (auto-rollup triggered)" } else { "" }
            ),
        }),
    ))
}

pub async fn get_balance(
    State(state): State<SharedState>,
    Path(account): Path<String>,
) -> Json<BalanceRes> {
    let l2 = state.l2.lock().await;
    Json(BalanceRes {
        balance: l2.state.balance(&account),
        account,
    })
}

pub async fn get_balances(State(state): State<SharedState>) -> Json<BalancesRes> {
    let l2 = state.l2.lock().await;
    Json(BalancesRes {
        balances: l2.state.balances.clone(),
        state_root: l2.state.state_root(),
    })
}

pub async fn get_pending(State(state): State<SharedState>) -> Json<PendingRes> {
    let l2 = state.l2.lock().await;
    Json(PendingRes {
        count: l2.pending_txs.len(),
        transactions: l2.pending_txs.clone(),
    })
}

pub async fn rollup(
    State(state): State<SharedState>,
) -> Result<(StatusCode, Json<RollupRes>), (StatusCode, Json<OkMsg>)> {
    let mut l2 = state.l2.lock().await;
    let mut bc = state.blockchain.lock().await;
    let r = l2
        .create_rollup(&mut bc)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(OkMsg { message: e })))?;
    Ok((
        StatusCode::CREATED,
        Json(RollupRes {
            message: format!(
                "rollup #{} submitted to L1 block {}",
                r.id, r.l1_block_index
            ),
            rollup: r,
        }),
    ))
}

pub async fn get_rollups(State(state): State<SharedState>) -> Json<RollupsRes> {
    let l2 = state.l2.lock().await;
    Json(RollupsRes {
        count: l2.rollups.len(),
        rollups: l2.rollups.clone(),
    })
}

pub async fn verify_proof(Json(body): Json<VerifyProofReq>) -> Json<VerifyRes> {
    Json(VerifyRes {
        valid: body.proof.verify(),
    })
}
