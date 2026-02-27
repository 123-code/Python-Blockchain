mod api;
mod blockchain;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(blockchain::Blockchain::new()));

    let app = Router::new()
        .route("/chain", get(api::get_chain))
        .route("/mine", post(api::mine_block))
        .route("/validate", get(api::validate_chain))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    println!("Blockchain node running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
