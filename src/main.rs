mod agent;
mod api;
mod blockchain;
mod explorer;
mod history;
mod l2;
mod vm;
mod wallet;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use api::AppState;
use history::History;
use l2::engine::L2Engine;
use vm::contract::ContractStore;
use wallet::WalletStore;

const L2_BATCH_SIZE: usize = 5;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        blockchain: Mutex::new(blockchain::Blockchain::new()),
        l2: Mutex::new(L2Engine::new(L2_BATCH_SIZE)),
        contracts: Mutex::new(ContractStore::new()),
        wallets: Mutex::new(WalletStore::new()),
        history: Mutex::new(History::new()),
    });

    let app = Router::new()
        // Explorer
        .route("/", get(explorer::dashboard))
        // Stats
        .route("/stats", get(history::get_stats))
        .route("/history", get(history::get_history))
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
        // Wallets
        .route("/wallet/new", post(wallet::create_wallet))
        .route("/wallet/sign", post(wallet::sign_message))
        .route("/wallet/verify", post(wallet::verify_signature))
        .route("/wallet/list", get(wallet::list_wallets))
        // AI Agent Protocol
        .route("/ai/schema", get(agent::get_schema))
        .route("/ai/execute", post(agent::execute))
        .route("/ai/batch", post(agent::batch))
        .route("/ai/command", post(agent::command))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    println!("Blockchain node running on http://{}", addr);
    println!("  Explorer:  http://{}/ (web dashboard)", addr);
    println!("  L1:        GET /chain, POST /mine, GET /validate");
    println!("  L2:        POST /l2/deposit, /l2/transfer, /l2/withdraw, /l2/rollup");
    println!("  Contracts: POST /contract/deploy, /contract/call");
    println!("  Wallets:   POST /wallet/new, /wallet/sign, /wallet/verify");
    println!("  AI Agent:  GET /ai/schema | POST /ai/command, /ai/execute, /ai/batch");
    println!("  Stats:     GET /stats, /history");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
