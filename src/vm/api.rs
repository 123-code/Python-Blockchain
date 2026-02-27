use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::contract::ContractInfo;
use super::executor::{ExecResult, DEFAULT_GAS_LIMIT};
use crate::api::SharedState;

#[derive(Deserialize)]
pub struct DeployReq {
    pub sender: String,
    pub code: String,
    pub init_args: Option<Vec<i64>>,
}

#[derive(Serialize)]
pub struct DeployRes {
    pub address: String,
    pub result: Option<ExecResult>,
}

#[derive(Deserialize)]
pub struct CallReq {
    pub sender: String,
    pub address: String,
    #[serde(default)]
    pub value: u64,
    #[serde(default)]
    pub args: Vec<i64>,
    pub gas_limit: Option<u64>,
}

#[derive(Serialize)]
pub struct ErrMsg {
    pub error: String,
}

pub async fn deploy(
    State(state): State<SharedState>,
    Json(body): Json<DeployReq>,
) -> Result<(StatusCode, Json<DeployRes>), (StatusCode, Json<ErrMsg>)> {
    let mut contracts = state.contracts.lock().await;
    let mut l2 = state.l2.lock().await;
    let block_height = {
        let bc = state.blockchain.lock().await;
        bc.chain.len() as u64
    };

    let (address, result) = contracts
        .deploy(&body.sender, &body.code, body.init_args, &mut l2.state, block_height)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrMsg { error: e })))?;

    Ok((StatusCode::CREATED, Json(DeployRes { address, result })))
}

pub async fn call(
    State(state): State<SharedState>,
    Json(body): Json<CallReq>,
) -> Result<Json<ExecResult>, (StatusCode, Json<ErrMsg>)> {
    let mut contracts = state.contracts.lock().await;
    let mut l2 = state.l2.lock().await;
    let block_height = {
        let bc = state.blockchain.lock().await;
        bc.chain.len() as u64
    };

    let gas = body.gas_limit.unwrap_or(DEFAULT_GAS_LIMIT);
    let result = contracts
        .call(
            &body.address,
            &body.sender,
            body.value,
            body.args,
            &mut l2.state,
            block_height,
            gas,
        )
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrMsg { error: e })))?;

    Ok(Json(result))
}

pub async fn get_contract(
    State(state): State<SharedState>,
    Path(address): Path<String>,
) -> Result<Json<ContractInfo>, (StatusCode, Json<ErrMsg>)> {
    let contracts = state.contracts.lock().await;
    contracts
        .get(&address)
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrMsg {
                    error: format!("contract not found: {}", address),
                }),
            )
        })
}

#[derive(Serialize)]
pub struct StorageVal {
    pub key: String,
    pub value: i64,
}

pub async fn get_storage(
    State(state): State<SharedState>,
    Path((address, key)): Path<(String, String)>,
) -> Result<Json<StorageVal>, (StatusCode, Json<ErrMsg>)> {
    let contracts = state.contracts.lock().await;
    let val = contracts.get_storage_value(&address, &key).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrMsg {
                error: "key not found".into(),
            }),
        )
    })?;
    Ok(Json(StorageVal { key, value: val }))
}

#[derive(Serialize)]
pub struct ContractListRes {
    pub count: usize,
    pub contracts: Vec<ContractInfo>,
}

pub async fn list_contracts(
    State(state): State<SharedState>,
) -> Json<ContractListRes> {
    let contracts = state.contracts.lock().await;
    let list: Vec<ContractInfo> = contracts
        .contracts
        .values()
        .map(|c| ContractInfo {
            address: c.address.clone(),
            deployer: c.deployer.clone(),
            balance: c.balance,
            storage: c.storage.clone(),
            source: c.source.clone(),
        })
        .collect();
    Json(ContractListRes {
        count: list.len(),
        contracts: list,
    })
}
