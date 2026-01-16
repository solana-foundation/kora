use super::{super::limiter::LimiterContext, InstructionRule, TransactionRule};

macro_rules! delegate {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            UsageRule::Transaction(r) => r.$method($($arg),*),
            UsageRule::Instruction(r) => r.$method($($arg),*),
        }
    };
}

#[derive(Debug)]
pub enum UsageRule {
    Transaction(TransactionRule),
    Instruction(InstructionRule),
}

impl UsageRule {
    pub fn rule_type(&self) -> &'static str {
        match self {
            Self::Transaction(_) => "transaction",
            Self::Instruction(_) => "instruction",
        }
    }

    pub fn storage_key(&self, user_id: &str, timestamp: u64) -> String {
        delegate!(self, storage_key, user_id, timestamp)
    }

    pub fn count_increment(&self, ctx: &mut LimiterContext<'_>) -> u64 {
        delegate!(self, count_increment, ctx)
    }

    pub fn max(&self) -> u64 {
        delegate!(self, max)
    }

    pub fn window_seconds(&self) -> Option<u64> {
        delegate!(self, window_seconds)
    }

    pub fn description(&self) -> String {
        delegate!(self, description)
    }

    pub fn as_instruction(&self) -> Option<&InstructionRule> {
        match self {
            Self::Instruction(r) => Some(r),
            Self::Transaction(_) => None,
        }
    }
}
