use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Push(i64),
    Pop,
    Dup,
    Swap,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Lt,
    Gt,
    Not,
    And,
    Or,
    Load(String),
    Store(String),
    Jump(usize),
    JumpIf(usize),
    Halt,
    Revert,
    Caller,
    CallValue,
    Balance,
    BlockHeight,
    Emit(String),
    Transfer(String),
}

impl Op {
    pub fn gas_cost(&self) -> u64 {
        match self {
            Self::Push(_) | Self::Pop | Self::Dup | Self::Swap => 1,
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::Mod => 3,
            Self::Eq | Self::Lt | Self::Gt | Self::Not | Self::And | Self::Or => 3,
            Self::Load(_) | Self::Store(_) => 5,
            Self::Jump(_) | Self::JumpIf(_) => 2,
            Self::Halt | Self::Revert => 0,
            Self::Caller | Self::CallValue | Self::Balance | Self::BlockHeight => 2,
            Self::Emit(_) => 5,
            Self::Transfer(_) => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_costs_non_zero_for_compute() {
        assert!(Op::Add.gas_cost() > 0);
        assert!(Op::Push(1).gas_cost() > 0);
        assert!(Op::Store("x".into()).gas_cost() > 0);
    }

    #[test]
    fn halt_revert_free() {
        assert_eq!(Op::Halt.gas_cost(), 0);
        assert_eq!(Op::Revert.gas_cost(), 0);
    }
}
