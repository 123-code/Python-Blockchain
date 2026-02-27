mod api;
mod blockchain;
mod l2;
mod vm;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use api::AppState;
use l2::engine::L2Engine;
use vm::contract::ContractStore;

const L2_BATCH_SIZE: usize = 5;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        blockchain: Mutex::new(blockchain::Blockchain::new()),
        l2: Mutex::new(L2Engine::new(L2_BATCH_SIZE)),
        contracts: Mutex::new(ContractStore::new()),
    });

    let app = Router::new()
        // L1
        .route("/chain", get(api::get_chain))
        .route("/mine", post(api::mine_block))
        .route("/validate", get(api::validate_chain))
        // L2
        .route("/l2/deposit", post(l2::api::deposit))
        .route("/l2/transfer", post(l2::api::transfer))
        .route("/l2/withdraw", post(l2::api::withdraw))
        .route("/l2/balance/:account", get(l2::api::get_balance))
        .route("/l2/balances", get(l2::api::get_balances))
        .route("/l2/pending", get(l2::api::get_pending))
        .route("/l2/rollup", post(l2::api::rollup))
        .route("/l2/rollups", get(l2::api::get_rollups))
        .route("/l2/verify", post(l2::api::verify_proof))
        // Contracts
        .route("/contract/deploy", post(vm::api::deploy))
        .route("/contract/call", post(vm::api::call))
        .route("/contract/list", get(vm::api::list_contracts))
        .route("/contract/:address", get(vm::api::get_contract))
        .route(
            "/contract/:address/storage/:key",
            get(vm::api::get_storage),
        )
        .with_state(state);

    let addr = "0.0.0.0:8080";
    println!("Blockchain node running on http://{}", addr);
    println!("  L1:        GET /chain, POST /mine, GET /validate");
    println!("  L2:        POST /l2/deposit, /l2/transfer, /l2/withdraw, /l2/rollup");
    println!("             GET  /l2/balance/:acct, /l2/balances, /l2/pending, /l2/rollups");
    println!("  Contracts: POST /contract/deploy, /contract/call");
    println!("             GET  /contract/list, /contract/:addr, /contract/:addr/storage/:key");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
