use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, Json};
use crate::api::SharedState;

/// Wallet using HMAC-SHA256. Production systems should use ed25519.
/// The signing model here is server-custodial for demo purposes.

#[derive(Clone, Serialize)]
pub struct WalletInfo {
    pub address: String,
    pub public_key: String,
}

pub struct WalletStore {
    keys: BTreeMap<String, (String, String)>, // address → (secret, public)
    pub wallets: Vec<WalletInfo>,
    nonce: u64,
}

impl WalletStore {
    pub fn new() -> Self {
        WalletStore {
            keys: BTreeMap::new(),
            wallets: Vec::new(),
            nonce: 0,
        }
    }

    pub fn create(&mut self) -> (WalletInfo, String) {
        self.nonce += 1;
        let entropy = format!(
            "keygen:{}:{}",
            self.nonce,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let secret = sha(&entropy);
        let public = sha(&secret);
        let address = format!("0x{}", &sha(&public)[..20]);

        let info = WalletInfo {
            address: address.clone(),
            public_key: public.clone(),
        };

        self.keys
            .insert(address.clone(), (secret.clone(), public));
        self.wallets.push(info.clone());
        (info, secret)
    }

    pub fn sign(&self, address: &str, message: &str) -> Result<String, String> {
        let (secret, _) = self
            .keys
            .get(address)
            .ok_or_else(|| format!("wallet not found: {}", address))?;
        Ok(hmac(secret, message))
    }

    pub fn verify(&self, address: &str, message: &str, signature: &str) -> Result<bool, String> {
        let (secret, _) = self
            .keys
            .get(address)
            .ok_or_else(|| format!("wallet not found: {}", address))?;
        Ok(hmac(secret, message) == signature)
    }
}

fn sha(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())
}

fn hmac(key: &str, message: &str) -> String {
    let mut h = Sha256::new();
    h.update(format!("hmac:{}:{}", key, message).as_bytes());
    hex::encode(h.finalize())
}

// ── API types & handlers ────────────────────────────────────────

#[derive(Serialize)]
pub struct CreateRes {
    pub address: String,
    pub public_key: String,
    pub secret_key: String,
}

#[derive(Deserialize)]
pub struct SignReq {
    pub address: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct SignRes {
    pub signature: String,
}

#[derive(Deserialize)]
pub struct VerifyReq {
    pub address: String,
    pub message: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct VerifyRes {
    pub valid: bool,
}

#[derive(Serialize)]
pub struct ErrMsg {
    pub error: String,
}

#[derive(Serialize)]
pub struct WalletListRes {
    pub count: usize,
    pub wallets: Vec<WalletInfo>,
}

pub async fn create_wallet(
    State(state): State<SharedState>,
) -> (StatusCode, Json<CreateRes>) {
    let mut w = state.wallets.lock().await;
    let (info, secret) = w.create();
    let mut h = state.history.lock().await;
    h.record(
        "wallet_create",
        &info.address,
        serde_json::json!({"address": &info.address}),
        true,
    );
    (
        StatusCode::CREATED,
        Json(CreateRes {
            address: info.address,
            public_key: info.public_key,
            secret_key: secret,
        }),
    )
}

pub async fn sign_message(
    State(state): State<SharedState>,
    Json(body): Json<SignReq>,
) -> Result<Json<SignRes>, (StatusCode, Json<ErrMsg>)> {
    let w = state.wallets.lock().await;
    let sig = w
        .sign(&body.address, &body.message)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrMsg { error: e })))?;
    Ok(Json(SignRes { signature: sig }))
}

pub async fn verify_signature(
    State(state): State<SharedState>,
    Json(body): Json<VerifyReq>,
) -> Result<Json<VerifyRes>, (StatusCode, Json<ErrMsg>)> {
    let w = state.wallets.lock().await;
    let valid = w
        .verify(&body.address, &body.message, &body.signature)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrMsg { error: e })))?;
    Ok(Json(VerifyRes { valid }))
}

pub async fn list_wallets(State(state): State<SharedState>) -> Json<WalletListRes> {
    let w = state.wallets.lock().await;
    Json(WalletListRes {
        count: w.wallets.len(),
        wallets: w.wallets.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_unique_wallets() {
        let mut s = WalletStore::new();
        let (a, _) = s.create();
        let (b, _) = s.create();
        assert_ne!(a.address, b.address);
        assert!(a.address.starts_with("0x"));
    }

    #[test]
    fn sign_and_verify() {
        let mut s = WalletStore::new();
        let (info, _) = s.create();
        let sig = s.sign(&info.address, "hello").unwrap();
        assert!(s.verify(&info.address, "hello", &sig).unwrap());
        assert!(!s.verify(&info.address, "wrong", &sig).unwrap());
    }

    #[test]
    fn different_messages_different_sigs() {
        let mut s = WalletStore::new();
        let (info, _) = s.create();
        let s1 = s.sign(&info.address, "a").unwrap();
        let s2 = s.sign(&info.address, "b").unwrap();
        assert_ne!(s1, s2);
    }

    #[test]
    fn unknown_wallet() {
        let s = WalletStore::new();
        assert!(s.sign("0xfake", "msg").is_err());
    }
}
