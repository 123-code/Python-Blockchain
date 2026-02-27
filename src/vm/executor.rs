use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::opcode::Op;

pub const DEFAULT_GAS_LIMIT: u64 = 100_000;
const MAX_STACK: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub value: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecResult {
    pub success: bool,
    pub gas_used: u64,
    pub return_value: Option<i64>,
    pub events: Vec<Event>,
    pub storage: BTreeMap<String, i64>,
    pub balance_changes: Vec<BalanceChange>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceChange {
    pub account: String,
    pub amount: u64,
    pub direction: String,
}

pub struct ExecContext {
    pub caller: String,
    pub value: u64,
    pub args: Vec<i64>,
    pub block_height: u64,
    pub contract_balance: u64,
}

/// Deterministic numeric ID for a string (for CALLER opcode).
pub fn str_to_id(s: &str) -> i64 {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let hash = h.finalize();
    let bytes: [u8; 8] = hash[..8].try_into().unwrap();
    i64::from_be_bytes(bytes).abs()
}

pub fn execute(
    code: &[Op],
    ctx: &ExecContext,
    storage: &mut BTreeMap<String, i64>,
    gas_limit: u64,
) -> ExecResult {
    let mut stack: Vec<i64> = Vec::new();
    let mut gas_used: u64 = 0;
    let mut events: Vec<Event> = Vec::new();
    let mut balance_changes: Vec<BalanceChange> = Vec::new();
    let mut contract_bal = ctx.contract_balance + ctx.value;
    let mut pc: usize = 0;

    // Push args in reverse so args[0] ends on top
    for arg in ctx.args.iter().rev() {
        stack.push(*arg);
    }

    macro_rules! err {
        ($msg:expr) => {
            return ExecResult {
                success: false,
                gas_used,
                return_value: None,
                events,
                storage: storage.clone(),
                balance_changes,
                error: Some($msg.to_string()),
            }
        };
    }

    macro_rules! pop {
        () => {
            match stack.pop() {
                Some(v) => v,
                None => err!("stack underflow"),
            }
        };
    }

    loop {
        if pc >= code.len() {
            err!("pc out of bounds (missing HALT?)");
        }

        let op = &code[pc];
        let cost = op.gas_cost();
        gas_used += cost;
        if gas_used > gas_limit {
            err!(format!("out of gas (limit {})", gas_limit));
        }

        match op {
            Op::Push(v) => {
                if stack.len() >= MAX_STACK {
                    err!("stack overflow");
                }
                stack.push(*v);
            }
            Op::Pop => {
                pop!();
            }
            Op::Dup => {
                let v = pop!();
                stack.push(v);
                stack.push(v);
            }
            Op::Swap => {
                let a = pop!();
                let b = pop!();
                stack.push(a);
                stack.push(b);
            }
            Op::Add => {
                let b = pop!();
                let a = pop!();
                stack.push(a.wrapping_add(b));
            }
            Op::Sub => {
                let b = pop!();
                let a = pop!();
                stack.push(a.wrapping_sub(b));
            }
            Op::Mul => {
                let b = pop!();
                let a = pop!();
                stack.push(a.wrapping_mul(b));
            }
            Op::Div => {
                let b = pop!();
                let a = pop!();
                if b == 0 {
                    err!("division by zero");
                }
                stack.push(a / b);
            }
            Op::Mod => {
                let b = pop!();
                let a = pop!();
                if b == 0 {
                    err!("modulo by zero");
                }
                stack.push(a % b);
            }
            Op::Eq => {
                let b = pop!();
                let a = pop!();
                stack.push(if a == b { 1 } else { 0 });
            }
            Op::Lt => {
                let b = pop!();
                let a = pop!();
                stack.push(if a < b { 1 } else { 0 });
            }
            Op::Gt => {
                let b = pop!();
                let a = pop!();
                stack.push(if a > b { 1 } else { 0 });
            }
            Op::Not => {
                let a = pop!();
                stack.push(if a == 0 { 1 } else { 0 });
            }
            Op::And => {
                let b = pop!();
                let a = pop!();
                stack.push(if a != 0 && b != 0 { 1 } else { 0 });
            }
            Op::Or => {
                let b = pop!();
                let a = pop!();
                stack.push(if a != 0 || b != 0 { 1 } else { 0 });
            }
            Op::Load(key) => {
                let v = storage.get(key).copied().unwrap_or(0);
                stack.push(v);
            }
            Op::Store(key) => {
                let v = pop!();
                storage.insert(key.clone(), v);
            }
            Op::Jump(target) => {
                pc = *target;
                continue;
            }
            Op::JumpIf(target) => {
                let cond = pop!();
                if cond != 0 {
                    pc = *target;
                    continue;
                }
            }
            Op::Halt => {
                return ExecResult {
                    success: true,
                    gas_used,
                    return_value: stack.last().copied(),
                    events,
                    storage: storage.clone(),
                    balance_changes,
                    error: None,
                };
            }
            Op::Revert => {
                err!("REVERT");
            }
            Op::Caller => {
                stack.push(str_to_id(&ctx.caller));
            }
            Op::CallValue => {
                stack.push(ctx.value as i64);
            }
            Op::Balance => {
                stack.push(contract_bal as i64);
            }
            Op::BlockHeight => {
                stack.push(ctx.block_height as i64);
            }
            Op::Emit(name) => {
                let v = pop!();
                events.push(Event {
                    name: name.clone(),
                    value: v,
                });
            }
            Op::Transfer(account) => {
                let amount = pop!();
                if amount < 0 {
                    err!("transfer: negative amount");
                }
                let amt = amount as u64;
                if amt > contract_bal {
                    err!(format!(
                        "transfer: insufficient contract balance ({} < {})",
                        contract_bal, amt
                    ));
                }
                contract_bal -= amt;
                balance_changes.push(BalanceChange {
                    account: account.clone(),
                    amount: amt,
                    direction: "credit".into(),
                });
            }
        }

        pc += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::compiler::compile;

    fn run(code: &str) -> ExecResult {
        run_with_args(code, vec![], 0)
    }

    fn run_with_args(code: &str, args: Vec<i64>, value: u64) -> ExecResult {
        let ops = compile(code).unwrap();
        let mut storage = BTreeMap::new();
        let ctx = ExecContext {
            caller: "alice".into(),
            value,
            args,
            block_height: 1,
            contract_balance: 1000,
        };
        execute(&ops, &ctx, &mut storage, DEFAULT_GAS_LIMIT)
    }

    #[test]
    fn arithmetic() {
        let r = run("PUSH 10\nPUSH 3\nADD\nHALT");
        assert!(r.success);
        assert_eq!(r.return_value, Some(13));
    }

    #[test]
    fn subtraction() {
        let r = run("PUSH 10\nPUSH 3\nSUB\nHALT");
        assert_eq!(r.return_value, Some(7));
    }

    #[test]
    fn multiplication() {
        let r = run("PUSH 6\nPUSH 7\nMUL\nHALT");
        assert_eq!(r.return_value, Some(42));
    }

    #[test]
    fn division() {
        let r = run("PUSH 20\nPUSH 4\nDIV\nHALT");
        assert_eq!(r.return_value, Some(5));
    }

    #[test]
    fn division_by_zero() {
        let r = run("PUSH 1\nPUSH 0\nDIV\nHALT");
        assert!(!r.success);
        assert!(r.error.unwrap().contains("division by zero"));
    }

    #[test]
    fn modulo() {
        let r = run("PUSH 17\nPUSH 5\nMOD\nHALT");
        assert_eq!(r.return_value, Some(2));
    }

    #[test]
    fn comparison() {
        let r = run("PUSH 5\nPUSH 10\nLT\nHALT");
        assert_eq!(r.return_value, Some(1));
        let r = run("PUSH 10\nPUSH 5\nLT\nHALT");
        assert_eq!(r.return_value, Some(0));
    }

    #[test]
    fn equality() {
        let r = run("PUSH 42\nPUSH 42\nEQ\nHALT");
        assert_eq!(r.return_value, Some(1));
        let r = run("PUSH 1\nPUSH 2\nEQ\nHALT");
        assert_eq!(r.return_value, Some(0));
    }

    #[test]
    fn logic_ops() {
        let r = run("PUSH 1\nPUSH 1\nAND\nHALT");
        assert_eq!(r.return_value, Some(1));
        let r = run("PUSH 0\nPUSH 1\nAND\nHALT");
        assert_eq!(r.return_value, Some(0));
        let r = run("PUSH 0\nPUSH 1\nOR\nHALT");
        assert_eq!(r.return_value, Some(1));
        let r = run("PUSH 0\nNOT\nHALT");
        assert_eq!(r.return_value, Some(1));
    }

    #[test]
    fn storage_load_store() {
        let r = run("PUSH 99\nSTORE val\nLOAD val\nHALT");
        assert!(r.success);
        assert_eq!(r.return_value, Some(99));
        assert_eq!(r.storage.get("val"), Some(&99));
    }

    #[test]
    fn load_default_zero() {
        let r = run("LOAD missing\nHALT");
        assert_eq!(r.return_value, Some(0));
    }

    #[test]
    fn loop_with_jump() {
        let code = "\
PUSH 0
STORE i
:loop
LOAD i
PUSH 1
ADD
STORE i
LOAD i
PUSH 5
LT
JUMPIF :loop
LOAD i
HALT";
        let r = run(code);
        assert!(r.success);
        assert_eq!(r.return_value, Some(5));
        assert_eq!(r.storage.get("i"), Some(&5));
    }

    #[test]
    fn out_of_gas() {
        let code = ":loop\nPUSH 1\nPOP\nJUMP :loop";
        let ops = compile(code).unwrap();
        let mut storage = BTreeMap::new();
        let ctx = ExecContext {
            caller: "a".into(),
            value: 0,
            args: vec![],
            block_height: 1,
            contract_balance: 0,
        };
        let r = execute(&ops, &ctx, &mut storage, 50);
        assert!(!r.success);
        assert!(r.error.unwrap().contains("out of gas"));
    }

    #[test]
    fn stack_underflow() {
        let r = run("POP\nHALT");
        assert!(!r.success);
        assert!(r.error.unwrap().contains("stack underflow"));
    }

    #[test]
    fn revert() {
        let r = run("PUSH 1\nREVERT");
        assert!(!r.success);
        assert!(r.error.unwrap().contains("REVERT"));
    }

    #[test]
    fn caller_opcode() {
        let r = run("CALLER\nHALT");
        assert!(r.success);
        assert_eq!(r.return_value, Some(str_to_id("alice")));
    }

    #[test]
    fn call_value_opcode() {
        let r = run_with_args("CALLVALUE\nHALT", vec![], 500);
        assert_eq!(r.return_value, Some(500));
    }

    #[test]
    fn balance_opcode() {
        let r = run("BALANCE\nHALT");
        // contract_balance=1000, value=0 → 1000
        assert_eq!(r.return_value, Some(1000));
    }

    #[test]
    fn emit_event() {
        let r = run("PUSH 42\nEMIT answer\nHALT");
        assert!(r.success);
        assert_eq!(r.events.len(), 1);
        assert_eq!(r.events[0].name, "answer");
        assert_eq!(r.events[0].value, 42);
    }

    #[test]
    fn transfer_opcode() {
        let r = run("PUSH 100\nTRANSFER bob\nHALT");
        assert!(r.success);
        assert_eq!(r.balance_changes.len(), 1);
        assert_eq!(r.balance_changes[0].account, "bob");
        assert_eq!(r.balance_changes[0].amount, 100);
    }

    #[test]
    fn transfer_insufficient() {
        let r = run("PUSH 9999\nTRANSFER bob\nHALT");
        assert!(!r.success);
        assert!(r.error.unwrap().contains("insufficient"));
    }

    #[test]
    fn args_on_stack() {
        let r = run_with_args("HALT", vec![10, 20, 30], 0);
        assert!(r.success);
        // args[0]=10 is on top
        assert_eq!(r.return_value, Some(10));
    }

    #[test]
    fn function_dispatch() {
        let code = "\
DUP
PUSH 1
EQ
JUMPIF :inc
DUP
PUSH 2
EQ
JUMPIF :read
REVERT
:inc
POP
LOAD counter
PUSH 1
ADD
STORE counter
LOAD counter
HALT
:read
POP
LOAD counter
HALT";
        // Call with function 1 (increment)
        let ops = compile(code).unwrap();
        let mut storage = BTreeMap::new();
        let ctx = ExecContext {
            caller: "a".into(),
            value: 0,
            args: vec![1],
            block_height: 1,
            contract_balance: 0,
        };
        let r = execute(&ops, &ctx, &mut storage, DEFAULT_GAS_LIMIT);
        assert!(r.success);
        assert_eq!(r.return_value, Some(1));

        // Call again with function 1
        let ctx2 = ExecContext { args: vec![1], ..ctx };
        let r2 = execute(&ops, &ctx2, &mut storage, DEFAULT_GAS_LIMIT);
        assert_eq!(r2.return_value, Some(2));

        // Call with function 2 (read)
        let ctx3 = ExecContext {
            caller: "a".into(),
            value: 0,
            args: vec![2],
            block_height: 1,
            contract_balance: 0,
        };
        let r3 = execute(&ops, &ctx3, &mut storage, DEFAULT_GAS_LIMIT);
        assert_eq!(r3.return_value, Some(2));
    }

    #[test]
    fn dup_and_swap() {
        let r = run("PUSH 5\nDUP\nADD\nHALT");
        assert_eq!(r.return_value, Some(10));
        let r = run("PUSH 1\nPUSH 2\nSWAP\nHALT");
        assert_eq!(r.return_value, Some(1));
    }
}
