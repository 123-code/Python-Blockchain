use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::api::SharedState;
use crate::vm::executor::DEFAULT_GAS_LIMIT;

// ═══════════════════════════════════════════════════════════════
//  Schema: machine-readable tool definitions for AI agents
//  Compatible with Claude tool_use, OpenAI function calling, MCP
// ═══════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct Schema {
    pub name: String,
    pub description: String,
    pub actions: Vec<ActionDef>,
}

#[derive(Serialize)]
pub struct ActionDef {
    pub id: String,
    pub description: String,
    pub params: Vec<ParamDef>,
    pub returns: String,
}

#[derive(Serialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

pub async fn get_schema() -> Json<Schema> {
    Json(Schema {
        name: "Rust Blockchain".into(),
        description: "L1 blockchain with L2 ZK-rollup scaling, smart contracts, and wallets".into(),
        actions: vec![
            ActionDef {
                id: "l1_mine".into(),
                description: "Mine a new L1 block with proof-of-work".into(),
                params: vec![p("data", "string", false, "Block data payload")],
                returns: "Block object with index, hash, proof".into(),
            },
            ActionDef {
                id: "l2_deposit".into(),
                description: "Deposit tokens to an L2 account".into(),
                params: vec![
                    p("account", "string", true, "Account name"),
                    p("amount", "integer", true, "Amount to deposit"),
                ],
                returns: "Confirmation message".into(),
            },
            ActionDef {
                id: "l2_transfer".into(),
                description: "Transfer tokens between L2 accounts".into(),
                params: vec![
                    p("from", "string", true, "Sender account"),
                    p("to", "string", true, "Recipient account"),
                    p("amount", "integer", true, "Amount to transfer"),
                ],
                returns: "Confirmation message".into(),
            },
            ActionDef {
                id: "l2_withdraw".into(),
                description: "Withdraw tokens from an L2 account".into(),
                params: vec![
                    p("account", "string", true, "Account name"),
                    p("amount", "integer", true, "Amount to withdraw"),
                ],
                returns: "Confirmation message".into(),
            },
            ActionDef {
                id: "l2_balance".into(),
                description: "Query balance of an L2 account".into(),
                params: vec![p("account", "string", true, "Account name")],
                returns: "Account balance".into(),
            },
            ActionDef {
                id: "l2_rollup".into(),
                description: "Create a ZK-proved rollup of pending L2 transactions onto L1".into(),
                params: vec![],
                returns: "Rollup proof and L1 block index".into(),
            },
            ActionDef {
                id: "contract_deploy".into(),
                description: "Deploy a smart contract with assembly code".into(),
                params: vec![
                    p("sender", "string", true, "Deployer account"),
                    p("code", "string", true, "Assembly source code"),
                ],
                returns: "Contract address".into(),
            },
            ActionDef {
                id: "contract_call".into(),
                description: "Call a deployed smart contract".into(),
                params: vec![
                    p("address", "string", true, "Contract address"),
                    p("sender", "string", true, "Caller account"),
                    p("args", "array<integer>", false, "Function arguments"),
                    p("value", "integer", false, "Tokens to send"),
                ],
                returns: "Execution result with return value, gas, events".into(),
            },
            ActionDef {
                id: "wallet_create".into(),
                description: "Generate a new cryptographic wallet keypair".into(),
                params: vec![],
                returns: "Address, public key, secret key".into(),
            },
            ActionDef {
                id: "get_stats".into(),
                description: "Get chain-wide statistics".into(),
                params: vec![],
                returns: "Block count, accounts, value, contracts, wallets, transactions".into(),
            },
            ActionDef {
                id: "get_chain".into(),
                description: "Get the full L1 blockchain".into(),
                params: vec![],
                returns: "Array of blocks".into(),
            },
            ActionDef {
                id: "get_balances".into(),
                description: "Get all L2 account balances".into(),
                params: vec![],
                returns: "Map of account balances and state root".into(),
            },
            ActionDef {
                id: "list_contracts".into(),
                description: "List all deployed smart contracts".into(),
                params: vec![],
                returns: "Array of contracts with addresses, storage, balances".into(),
            },
        ],
    })
}

fn p(name: &str, t: &str, required: bool, desc: &str) -> ParamDef {
    ParamDef {
        name: name.into(),
        param_type: t.into(),
        required,
        description: desc.into(),
    }
}

// ═══════════════════════════════════════════════════════════════
//  Execute: structured action execution
// ═══════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct ExecReq {
    pub action: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ExecRes {
    pub success: bool,
    pub action: String,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

pub async fn execute(
    State(state): State<SharedState>,
    Json(body): Json<ExecReq>,
) -> Json<ExecRes> {
    let params = body.params.unwrap_or(serde_json::json!({}));
    match run_action(&state, &body.action, &params).await {
        Ok(result) => Json(ExecRes {
            success: true,
            action: body.action,
            result,
            error: None,
        }),
        Err(e) => Json(ExecRes {
            success: false,
            action: body.action,
            result: serde_json::json!(null),
            error: Some(e),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════
//  Batch: execute multiple actions in sequence
// ═══════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct BatchReq {
    pub actions: Vec<ExecReq>,
}

#[derive(Serialize)]
pub struct BatchRes {
    pub results: Vec<ExecRes>,
    pub all_succeeded: bool,
}

pub async fn batch(
    State(state): State<SharedState>,
    Json(body): Json<BatchReq>,
) -> Json<BatchRes> {
    let mut results = Vec::new();
    let mut all_ok = true;
    for action in &body.actions {
        let params = action.params.clone().unwrap_or(serde_json::json!({}));
        match run_action(&state, &action.action, &params).await {
            Ok(result) => results.push(ExecRes {
                success: true,
                action: action.action.clone(),
                result,
                error: None,
            }),
            Err(e) => {
                all_ok = false;
                results.push(ExecRes {
                    success: false,
                    action: action.action.clone(),
                    result: serde_json::json!(null),
                    error: Some(e),
                });
            }
        }
    }
    Json(BatchRes {
        results,
        all_succeeded: all_ok,
    })
}

// ═══════════════════════════════════════════════════════════════
//  Natural language command parser
// ═══════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct CommandReq {
    pub command: String,
}

#[derive(Serialize)]
pub struct CommandRes {
    pub input: String,
    pub parsed_action: String,
    pub parsed_params: serde_json::Value,
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

pub async fn command(
    State(state): State<SharedState>,
    Json(body): Json<CommandReq>,
) -> Json<CommandRes> {
    let input = body.command.trim().to_string();
    match parse_command(&input) {
        Ok((action, params)) => {
            let (success, result, error) = match run_action(&state, &action, &params).await {
                Ok(r) => (true, r, None),
                Err(e) => (false, serde_json::json!(null), Some(e)),
            };
            Json(CommandRes {
                input,
                parsed_action: action,
                parsed_params: params,
                success,
                result,
                error,
            })
        }
        Err(e) => Json(CommandRes {
            input,
            parsed_action: String::new(),
            parsed_params: serde_json::json!(null),
            success: false,
            result: serde_json::json!(null),
            error: Some(e),
        }),
    }
}

fn parse_command(input: &str) -> Result<(String, serde_json::Value), String> {
    let s = input.trim();
    let low = s.to_lowercase();
    let words: Vec<&str> = low.split_whitespace().collect();

    if words.is_empty() {
        return Err("empty command".into());
    }

    // ── deposit ──
    // "deposit 500 to alice", "deposit 500 alice", "give alice 500"
    if low.contains("deposit") || (words.len() >= 3 && words[0] == "give") {
        let (account, amount) = extract_account_amount(&low, &words)?;
        return Ok(("l2_deposit".into(), serde_json::json!({"account": account, "amount": amount})));
    }

    // ── transfer / send ──
    // "transfer 100 from alice to bob", "send 100 from alice to bob"
    // "send alice 100 to bob", "alice sends 100 to bob"
    if low.contains("transfer") || low.contains("send") {
        let (from, to, amount) = extract_transfer(&low, &words)?;
        return Ok(("l2_transfer".into(), serde_json::json!({"from": from, "to": to, "amount": amount})));
    }

    // ── withdraw ──
    // "withdraw 100 from alice"
    if low.contains("withdraw") {
        let (account, amount) = extract_account_amount(&low, &words)?;
        return Ok(("l2_withdraw".into(), serde_json::json!({"account": account, "amount": amount})));
    }

    // ── mine ──
    // "mine", "mine a block", "mine block with hello"
    if words[0] == "mine" {
        let data = if words.len() > 1 {
            let rest = s.splitn(2, ' ').nth(1).unwrap_or("").trim();
            let rest = rest
                .trim_start_matches("a ")
                .trim_start_matches("block ")
                .trim_start_matches("with data ")
                .trim_start_matches("with ")
                .trim();
            if rest.is_empty() { "Mined via command" } else { rest }
        } else {
            "Mined via command"
        };
        return Ok(("l1_mine".into(), serde_json::json!({"data": data})));
    }

    // ── balance ──
    // "balance of alice", "alice balance", "how much does alice have",
    // "check alice", "what is alice's balance"
    if low.contains("balance") || low.starts_with("check ") || low.contains("how much") {
        let account = extract_name(&low, &words)?;
        return Ok(("l2_balance".into(), serde_json::json!({"account": account})));
    }

    // ── rollup ──
    if low.contains("rollup") || low == "commit" || low.contains("prove") {
        return Ok(("l2_rollup".into(), serde_json::json!({})));
    }

    // ── wallet ──
    if low.contains("wallet") || low.contains("keypair") || low.contains("key pair") {
        return Ok(("wallet_create".into(), serde_json::json!({})));
    }

    // ── deploy contract ──
    // "deploy contract as alice: PUSH 1\nSTORE x\nHALT"
    // "deploy PUSH 1\nHALT sender alice"
    if low.starts_with("deploy") {
        let rest = s.splitn(2, ' ').nth(1).unwrap_or("").trim();
        let rest = rest.trim_start_matches("contract ").trim();
        let (sender, code) = if let Some(pos) = rest.find(':') {
            let before = rest[..pos].trim();
            let after = rest[pos + 1..].trim();
            let sender = before
                .trim_start_matches("as ")
                .trim_start_matches("by ")
                .trim();
            (sender, after)
        } else if rest.contains(" sender ") {
            let parts: Vec<&str> = rest.rsplitn(2, " sender ").collect();
            (parts[0].trim(), parts[1].trim())
        } else {
            ("deployer", rest)
        };
        if code.is_empty() {
            return Err("deploy requires code after ':'".into());
        }
        return Ok(("contract_deploy".into(), serde_json::json!({"sender": sender, "code": code})));
    }

    // ── call contract ──
    // "call 0xaddr function 1 as alice"
    if words[0] == "call" && words.len() >= 2 {
        let address = words[1].to_string();
        let sender = words.iter().position(|&w| w == "as").map(|i| words.get(i + 1).unwrap_or(&"caller")).unwrap_or(&"caller").to_string();
        let args: Vec<i64> = words.iter()
            .filter_map(|w| w.parse::<i64>().ok())
            .collect();
        return Ok(("contract_call".into(), serde_json::json!({"address": address, "sender": sender, "args": args})));
    }

    // ── queries ──
    if low.contains("stats") || low.contains("status") || low == "info" {
        return Ok(("get_stats".into(), serde_json::json!({})));
    }
    if low.contains("chain") || low.contains("blocks") || low.starts_with("show block") {
        return Ok(("get_chain".into(), serde_json::json!({})));
    }
    if low.contains("balances") || low.contains("accounts") || low.starts_with("show account") {
        return Ok(("get_balances".into(), serde_json::json!({})));
    }
    if low.contains("contract") && (low.contains("list") || low.contains("show")) {
        return Ok(("list_contracts".into(), serde_json::json!({})));
    }
    if low.contains("history") || low.contains("transactions") || low.contains("log") {
        return Ok(("get_history".into(), serde_json::json!({})));
    }
    if low.contains("help") || low == "?" {
        return Ok(("help".into(), serde_json::json!({})));
    }

    Err(format!(
        "Could not understand '{}'. Try: deposit, transfer, send, withdraw, mine, balance, rollup, deploy, call, stats, wallet, help",
        s
    ))
}

// ── extraction helpers ──────────────────────────────────────────

fn extract_account_amount(low: &str, words: &[&str]) -> Result<(String, u64), String> {
    let amount = find_number(words)?;
    let account = words
        .iter()
        .filter(|w| {
            w.parse::<u64>().is_err()
                && !["deposit", "withdraw", "to", "from", "give", "into", "for", "tokens", "token"].contains(w)
        })
        .last()
        .ok_or_else(|| format!("could not find account name in '{}'", low))?;
    Ok((account.to_string(), amount))
}

fn extract_transfer(low: &str, words: &[&str]) -> Result<(String, String, u64), String> {
    let amount = find_number(words)?;
    let noise = ["transfer", "send", "sends", "to", "from", "tokens", "token"];

    // Try "from X to Y" pattern
    if let (Some(fi), Some(ti)) = (
        words.iter().position(|&w| w == "from"),
        words.iter().position(|&w| w == "to"),
    ) {
        let from = words.get(fi + 1).ok_or("missing sender after 'from'")?;
        let to = words.get(ti + 1).ok_or("missing recipient after 'to'")?;
        return Ok((from.to_string(), to.to_string(), amount));
    }

    // Try names by filtering noise
    let names: Vec<&&str> = words
        .iter()
        .filter(|w| w.parse::<u64>().is_err() && !noise.contains(w))
        .collect();
    if names.len() >= 2 {
        return Ok((names[0].to_string(), names[1].to_string(), amount));
    }
    Err(format!("could not parse transfer from '{}'", low))
}

fn extract_name(low: &str, words: &[&str]) -> Result<String, String> {
    let noise = [
        "balance", "of", "check", "how", "much", "does", "have", "what", "is", "show", "get", "the",
    ];
    let name = words
        .iter()
        .filter(|w| !noise.contains(w) && !w.contains('\''))
        .find(|w| w.parse::<u64>().is_err())
        .ok_or_else(|| format!("could not find account name in '{}'", low))?;
    Ok(name.to_string())
}

fn find_number(words: &[&str]) -> Result<u64, String> {
    words
        .iter()
        .find_map(|w| w.parse::<u64>().ok())
        .ok_or_else(|| "could not find a number in the command".into())
}

// ═══════════════════════════════════════════════════════════════
//  Action runner: executes parsed actions against state
// ═══════════════════════════════════════════════════════════════

async fn run_action(
    state: &SharedState,
    action: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match action {
        "l1_mine" => {
            let data = params["data"].as_str().unwrap_or("Mined via agent");
            let mut bc = state.blockchain.lock().await;
            let block = bc.mine_block(data.to_string());
            let mut h = state.history.lock().await;
            h.record("l1_mine", "agent", serde_json::json!({"block": block.index}), true);
            Ok(serde_json::json!({"block_index": block.index, "hash": block.hash, "proof": block.proof}))
        }
        "l2_deposit" => {
            let account = params["account"].as_str().ok_or("missing param: account")?;
            let amount = params["amount"].as_u64().ok_or("missing param: amount")?;
            let mut l2 = state.l2.lock().await;
            l2.submit_deposit(account, amount)?;
            if l2.should_auto_rollup() {
                let mut bc = state.blockchain.lock().await;
                let _ = l2.create_rollup(&mut bc);
            }
            let mut h = state.history.lock().await;
            h.record("l2_deposit", account, serde_json::json!({"amount": amount}), true);
            Ok(serde_json::json!({"message": format!("deposited {} to {}", amount, account)}))
        }
        "l2_transfer" => {
            let from = params["from"].as_str().ok_or("missing param: from")?;
            let to = params["to"].as_str().ok_or("missing param: to")?;
            let amount = params["amount"].as_u64().ok_or("missing param: amount")?;
            let mut l2 = state.l2.lock().await;
            l2.submit_transfer(from, to, amount)?;
            if l2.should_auto_rollup() {
                let mut bc = state.blockchain.lock().await;
                let _ = l2.create_rollup(&mut bc);
            }
            let mut h = state.history.lock().await;
            h.record("l2_transfer", from, serde_json::json!({"to": to, "amount": amount}), true);
            Ok(serde_json::json!({"message": format!("transferred {} from {} to {}", amount, from, to)}))
        }
        "l2_withdraw" => {
            let account = params["account"].as_str().ok_or("missing param: account")?;
            let amount = params["amount"].as_u64().ok_or("missing param: amount")?;
            let mut l2 = state.l2.lock().await;
            l2.submit_withdraw(account, amount)?;
            let mut h = state.history.lock().await;
            h.record("l2_withdraw", account, serde_json::json!({"amount": amount}), true);
            Ok(serde_json::json!({"message": format!("withdrew {} from {}", amount, account)}))
        }
        "l2_balance" => {
            let account = params["account"].as_str().ok_or("missing param: account")?;
            let l2 = state.l2.lock().await;
            let bal = l2.state.balance(account);
            Ok(serde_json::json!({"account": account, "balance": bal}))
        }
        "l2_rollup" => {
            let mut l2 = state.l2.lock().await;
            let mut bc = state.blockchain.lock().await;
            let r = l2.create_rollup(&mut bc)?;
            let mut h = state.history.lock().await;
            h.record("l2_rollup", "system", serde_json::json!({"id": r.id, "txs": r.tx_count}), true);
            Ok(serde_json::json!({"rollup_id": r.id, "tx_count": r.tx_count, "l1_block": r.l1_block_index, "proof_valid": r.proof.verify()}))
        }
        "contract_deploy" => {
            let sender = params["sender"].as_str().ok_or("missing param: sender")?;
            let code = params["code"].as_str().ok_or("missing param: code")?;
            let mut contracts = state.contracts.lock().await;
            let mut l2 = state.l2.lock().await;
            let block_height = { state.blockchain.lock().await.chain.len() as u64 };
            let (addr, _) = contracts.deploy(sender, code, None, &mut l2.state, block_height)?;
            let mut h = state.history.lock().await;
            h.record("contract_deploy", sender, serde_json::json!({"address": &addr}), true);
            Ok(serde_json::json!({"address": addr}))
        }
        "contract_call" => {
            let address = params["address"].as_str().ok_or("missing param: address")?;
            let sender = params["sender"].as_str().unwrap_or("caller");
            let value = params["value"].as_u64().unwrap_or(0);
            let args: Vec<i64> = params["args"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default();
            let mut contracts = state.contracts.lock().await;
            let mut l2 = state.l2.lock().await;
            let block_height = { state.blockchain.lock().await.chain.len() as u64 };
            let result = contracts.call(address, sender, value, args, &mut l2.state, block_height, DEFAULT_GAS_LIMIT)?;
            Ok(serde_json::json!({"success": result.success, "return_value": result.return_value, "gas_used": result.gas_used, "events": result.events}))
        }
        "wallet_create" => {
            let mut w = state.wallets.lock().await;
            let (info, secret) = w.create();
            let mut h = state.history.lock().await;
            h.record("wallet_create", &info.address, serde_json::json!({}), true);
            Ok(serde_json::json!({"address": info.address, "public_key": info.public_key, "secret_key": secret}))
        }
        "get_stats" => {
            let bc = state.blockchain.lock().await;
            let l2 = state.l2.lock().await;
            let c = state.contracts.lock().await;
            let w = state.wallets.lock().await;
            let h = state.history.lock().await;
            Ok(serde_json::json!({
                "l1_blocks": bc.chain.len(), "l1_valid": bc.is_chain_valid(),
                "l2_accounts": l2.state.balances.len(), "l2_total_value": l2.state.balances.values().sum::<u64>(),
                "l2_pending": l2.pending_txs.len(), "rollups": l2.rollups.len(),
                "contracts": c.contracts.len(), "wallets": w.wallets.len(),
                "transactions": h.count()
            }))
        }
        "get_chain" => {
            let bc = state.blockchain.lock().await;
            Ok(serde_json::json!({"length": bc.chain.len(), "blocks": bc.chain}))
        }
        "get_balances" => {
            let l2 = state.l2.lock().await;
            Ok(serde_json::json!({"balances": l2.state.balances, "state_root": l2.state.state_root()}))
        }
        "list_contracts" => {
            let c = state.contracts.lock().await;
            let list: Vec<serde_json::Value> = c.contracts.values().map(|ct| {
                serde_json::json!({"address": ct.address, "deployer": ct.deployer, "balance": ct.balance, "storage": ct.storage})
            }).collect();
            Ok(serde_json::json!({"count": list.len(), "contracts": list}))
        }
        "get_history" => {
            let h = state.history.lock().await;
            let records: Vec<_> = h.recent(50).into_iter().cloned().collect();
            Ok(serde_json::json!({"total": h.count(), "records": records}))
        }
        "help" => {
            Ok(serde_json::json!({
                "commands": [
                    "deposit <amount> to <account>",
                    "send/transfer <amount> from <account> to <account>",
                    "withdraw <amount> from <account>",
                    "balance of <account>",
                    "mine [block data]",
                    "rollup",
                    "create wallet",
                    "deploy as <sender>: <assembly code>",
                    "call <address> <args> as <sender>",
                    "stats / status",
                    "show chain / blocks / balances / contracts / history"
                ]
            }))
        }
        _ => Err(format!("unknown action: {}", action)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_deposit() {
        let (a, p) = parse_command("deposit 500 to alice").unwrap();
        assert_eq!(a, "l2_deposit");
        assert_eq!(p["account"], "alice");
        assert_eq!(p["amount"], 500);
    }

    #[test]
    fn parse_deposit_no_to() {
        let (a, p) = parse_command("deposit 100 bob").unwrap();
        assert_eq!(a, "l2_deposit");
        assert_eq!(p["account"], "bob");
        assert_eq!(p["amount"], 100);
    }

    #[test]
    fn parse_transfer() {
        let (a, p) = parse_command("transfer 200 from alice to bob").unwrap();
        assert_eq!(a, "l2_transfer");
        assert_eq!(p["from"], "alice");
        assert_eq!(p["to"], "bob");
        assert_eq!(p["amount"], 200);
    }

    #[test]
    fn parse_send() {
        let (a, p) = parse_command("send 50 from bob to charlie").unwrap();
        assert_eq!(a, "l2_transfer");
        assert_eq!(p["from"], "bob");
        assert_eq!(p["to"], "charlie");
    }

    #[test]
    fn parse_withdraw() {
        let (a, p) = parse_command("withdraw 100 from alice").unwrap();
        assert_eq!(a, "l2_withdraw");
        assert_eq!(p["account"], "alice");
        assert_eq!(p["amount"], 100);
    }

    #[test]
    fn parse_mine() {
        let (a, _) = parse_command("mine").unwrap();
        assert_eq!(a, "l1_mine");
        let (a, p) = parse_command("mine hello world").unwrap();
        assert_eq!(a, "l1_mine");
        assert_eq!(p["data"], "hello world");
    }

    #[test]
    fn parse_balance() {
        let (a, p) = parse_command("balance of alice").unwrap();
        assert_eq!(a, "l2_balance");
        assert_eq!(p["account"], "alice");
    }

    #[test]
    fn parse_balance_check() {
        let (a, p) = parse_command("check bob").unwrap();
        assert_eq!(a, "l2_balance");
        assert_eq!(p["account"], "bob");
    }

    #[test]
    fn parse_rollup() {
        let (a, _) = parse_command("rollup").unwrap();
        assert_eq!(a, "l2_rollup");
        let (a, _) = parse_command("create rollup").unwrap();
        assert_eq!(a, "l2_rollup");
    }

    #[test]
    fn parse_wallet() {
        let (a, _) = parse_command("create wallet").unwrap();
        assert_eq!(a, "wallet_create");
    }

    #[test]
    fn parse_stats() {
        let (a, _) = parse_command("show stats").unwrap();
        assert_eq!(a, "get_stats");
    }

    #[test]
    fn parse_help() {
        let (a, _) = parse_command("help").unwrap();
        assert_eq!(a, "help");
    }

    #[test]
    fn parse_deploy() {
        let (a, p) = parse_command("deploy as alice: PUSH 1\nHALT").unwrap();
        assert_eq!(a, "contract_deploy");
        assert_eq!(p["sender"], "alice");
        assert!(p["code"].as_str().unwrap().contains("PUSH"));
    }

    #[test]
    fn parse_unknown() {
        assert!(parse_command("foobar baz").is_err());
    }

    #[test]
    fn parse_empty() {
        assert!(parse_command("").is_err());
    }
}
