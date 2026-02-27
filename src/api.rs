use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::blockchain::{Block, Blockchain};
use crate::l2::engine::L2Engine;

pub struct AppState {
    pub blockchain: Mutex<Blockchain>,
    pub l2: Mutex<L2Engine>,
}

pub type SharedState = Arc<AppState>;

#[derive(Serialize)]
pub struct ChainResponse {
    pub chain: Vec<Block>,
    pub length: usize,
}

#[derive(Serialize)]
pub struct MineResponse {
    pub message: String,
    pub block: Block,
}

#[derive(Deserialize)]
pub struct MineRequest {
    pub data: Option<String>,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub length: usize,
}

pub async fn get_chain(State(state): State<SharedState>) -> Json<ChainResponse> {
    let bc = state.blockchain.lock().await;
    Json(ChainResponse {
        length: bc.chain.len(),
        chain: bc.chain.clone(),
    })
}

pub async fn mine_block(
    State(state): State<SharedState>,
    Json(body): Json<MineRequest>,
) -> (StatusCode, Json<MineResponse>) {
    let mut bc = state.blockchain.lock().await;
    let data = body.data.unwrap_or_else(|| "No data".into());
    let block = bc.mine_block(data);
    (
        StatusCode::CREATED,
        Json(MineResponse {
            message: "Block mined successfully".into(),
            block,
        }),
    )
}

pub async fn validate_chain(State(state): State<SharedState>) -> Json<ValidateResponse> {
    let bc = state.blockchain.lock().await;
    Json(ValidateResponse {
        valid: bc.is_chain_valid(),
        length: bc.chain.len(),
    })
}
