use std::collections::HashMap;

use super::opcode::Op;

/// Compile assembly text into a list of opcodes.
/// Supports labels (`:label_name`) and label references
/// in JUMP/JUMPIF (e.g. `JUMPIF :label_name`).
pub fn compile(source: &str) -> Result<Vec<Op>, String> {
    let lines = strip(source);

    // First pass: collect label positions
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut instruction_index = 0;
    for line in &lines {
        if let Some(name) = line.strip_prefix(':') {
            labels.insert(name.to_string(), instruction_index);
        } else {
            instruction_index += 1;
        }
    }

    // Second pass: parse instructions, resolve labels
    let mut ops = Vec::new();
    for line in &lines {
        if line.starts_with(':') {
            continue;
        }
        let op = parse_instruction(line, &labels)?;
        ops.push(op);
    }

    Ok(ops)
}

fn strip(source: &str) -> Vec<String> {
    source
        .lines()
        .map(|l| {
            let l = l.trim();
            if let Some(pos) = l.find(';') {
                l[..pos].trim()
            } else {
                l
            }
        })
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

fn parse_instruction(line: &str, labels: &HashMap<String, usize>) -> Result<Op, String> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    let cmd = parts[0].to_uppercase();
    let arg = parts.get(1).map(|s| s.trim());

    match cmd.as_str() {
        "PUSH" => {
            let v = arg
                .ok_or("PUSH requires a value")?
                .parse::<i64>()
                .map_err(|e| format!("PUSH: {}", e))?;
            Ok(Op::Push(v))
        }
        "POP" => Ok(Op::Pop),
        "DUP" => Ok(Op::Dup),
        "SWAP" => Ok(Op::Swap),
        "ADD" => Ok(Op::Add),
        "SUB" => Ok(Op::Sub),
        "MUL" => Ok(Op::Mul),
        "DIV" => Ok(Op::Div),
        "MOD" => Ok(Op::Mod),
        "EQ" => Ok(Op::Eq),
        "LT" => Ok(Op::Lt),
        "GT" => Ok(Op::Gt),
        "NOT" => Ok(Op::Not),
        "AND" => Ok(Op::And),
        "OR" => Ok(Op::Or),
        "LOAD" => Ok(Op::Load(require_arg("LOAD", arg)?)),
        "STORE" => Ok(Op::Store(require_arg("STORE", arg)?)),
        "JUMP" => Ok(Op::Jump(resolve_target("JUMP", arg, labels)?)),
        "JUMPIF" => Ok(Op::JumpIf(resolve_target("JUMPIF", arg, labels)?)),
        "HALT" => Ok(Op::Halt),
        "REVERT" => Ok(Op::Revert),
        "CALLER" => Ok(Op::Caller),
        "CALLVALUE" => Ok(Op::CallValue),
        "BALANCE" => Ok(Op::Balance),
        "BLOCKHEIGHT" => Ok(Op::BlockHeight),
        "EMIT" => Ok(Op::Emit(require_arg("EMIT", arg)?)),
        "TRANSFER" => Ok(Op::Transfer(require_arg("TRANSFER", arg)?)),
        _ => Err(format!("unknown opcode: {}", cmd)),
    }
}

fn require_arg(op: &str, arg: Option<&str>) -> Result<String, String> {
    arg.map(String::from)
        .ok_or_else(|| format!("{} requires an argument", op))
}

fn resolve_target(
    op: &str,
    arg: Option<&str>,
    labels: &HashMap<String, usize>,
) -> Result<usize, String> {
    let raw = arg.ok_or_else(|| format!("{} requires a target", op))?;
    if let Some(label) = raw.strip_prefix(':') {
        labels
            .get(label)
            .copied()
            .ok_or_else(|| format!("{}: undefined label '{}'", op, label))
    } else {
        raw.parse::<usize>()
            .map_err(|e| format!("{}: {}", op, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_basic() {
        let code = "PUSH 42\nPUSH 8\nADD\nHALT";
        let ops = compile(code).unwrap();
        assert_eq!(ops, vec![Op::Push(42), Op::Push(8), Op::Add, Op::Halt]);
    }

    #[test]
    fn compile_with_labels() {
        let code = "\
PUSH 0
STORE counter
:loop
LOAD counter
PUSH 1
ADD
STORE counter
LOAD counter
PUSH 5
LT
JUMPIF :loop
HALT";
        let ops = compile(code).unwrap();
        assert_eq!(ops[ops.len() - 2], Op::JumpIf(2)); // :loop → instruction 2
    }

    #[test]
    fn compile_comments_and_blanks() {
        let code = "\
; initialize
PUSH 1

; store it
STORE x
HALT
";
        let ops = compile(code).unwrap();
        assert_eq!(ops, vec![Op::Push(1), Op::Store("x".into()), Op::Halt]);
    }

    #[test]
    fn compile_storage_ops() {
        let code = "PUSH 99\nSTORE val\nLOAD val\nHALT";
        let ops = compile(code).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Push(99),
                Op::Store("val".into()),
                Op::Load("val".into()),
                Op::Halt,
            ]
        );
    }

    #[test]
    fn compile_unknown_opcode() {
        assert!(compile("FOOBAR").is_err());
    }

    #[test]
    fn compile_undefined_label() {
        assert!(compile("JUMP :nowhere").is_err());
    }

    #[test]
    fn compile_transfer_emit() {
        let code = "PUSH 50\nTRANSFER bob\nPUSH 50\nEMIT sent\nHALT";
        let ops = compile(code).unwrap();
        assert_eq!(ops[1], Op::Transfer("bob".into()));
        assert_eq!(ops[3], Op::Emit("sent".into()));
    }
}
