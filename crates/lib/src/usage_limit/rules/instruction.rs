use std::collections::HashMap;

use solana_sdk::pubkey::Pubkey;
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
use spl_associated_token_account_interface::program::ID as ATA_PROGRAM_ID;

use crate::transaction::{ParsedSystemInstructionData, ParsedSystemInstructionType};

use super::super::limiter::LimiterContext;

const IX_KEY_PREFIX: &str = "kora:ix";

/// Rule that limits specific instruction types per wallet
///
/// Counts matching instructions in each transaction and enforces limits.
/// Supports both lifetime limits (never resets) and time-windowed limits (resets periodically).
///
/// Currently supported instruction types:
/// - System: CreateAccount
/// - ATA: CreateIdempotent
#[derive(Debug)]
pub struct InstructionRule {
    program: Pubkey,
    instruction: String,
    max: u64,
    window_seconds: Option<u64>,
}

impl InstructionRule {
    pub fn new(
        program: Pubkey,
        instruction: String,
        max: u64,
        window_seconds: Option<u64>,
    ) -> Self {
        let lowered = instruction.to_lowercase();
        Self { program, instruction: lowered, max, window_seconds }
    }

    /// Create a lifetime instruction limit (never resets)
    pub fn lifetime(program: Pubkey, instruction: String, max: u64) -> Self {
        Self::new(program, instruction, max, None)
    }

    /// Create a time-windowed instruction limit
    pub fn windowed(program: Pubkey, instruction: String, max: u64, window_seconds: u64) -> Self {
        Self::new(program, instruction, max, Some(window_seconds))
    }

    /// Count matching instructions for one or more rules in a single pass
    /// Only counts instructions where Kora is the payer (subsidized operations)
    pub fn count_all_rules(rules: &[&InstructionRule], ctx: &mut LimiterContext<'_>) -> Vec<u64> {
        if rules.is_empty() {
            return vec![];
        }

        // Group rules by program ID
        let mut system_rules: Vec<(usize, &InstructionRule)> = vec![];
        let mut ata_rules: Vec<(usize, &InstructionRule)> = vec![];
        let mut other_rules: Vec<(usize, &InstructionRule)> = vec![];

        for (idx, rule) in rules.iter().enumerate() {
            if rule.program == SYSTEM_PROGRAM_ID {
                system_rules.push((idx, rule));
            } else if rule.program == ATA_PROGRAM_ID {
                ata_rules.push((idx, rule));
            } else {
                other_rules.push((idx, rule));
            }
        }

        let mut counts = vec![0u64; rules.len()];

        // Count System instructions
        if !system_rules.is_empty() {
            match ctx.transaction.get_or_parse_system_instructions() {
                Ok(parsed) => {
                    let kora_signer = ctx.kora_signer;
                    Self::count_batch_system_instructions(
                        &system_rules,
                        parsed,
                        kora_signer,
                        &mut counts,
                    );
                }
                Err(_) => {
                    Self::count_batch_manual(&system_rules, ctx, &mut counts);
                }
            }
        }

        // Count ATA instructions (manual parsing)
        if !ata_rules.is_empty() {
            Self::count_batch_manual(&ata_rules, ctx, &mut counts);
        }

        // Count other program instructions (manual parsing)
        if !other_rules.is_empty() {
            Self::count_batch_manual(&other_rules, ctx, &mut counts);
        }

        counts
    }

    /// Batch count system instructions for multiple rules
    /// Only counts instructions where Kora is the payer (subsidized operations)
    fn count_batch_system_instructions(
        rules: &[(usize, &InstructionRule)],
        parsed: &HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>,
        kora_signer: Option<Pubkey>,
        counts: &mut [u64],
    ) {
        for (idx, rule) in rules {
            let matching_type = match rule.instruction.as_str() {
                "createaccount" | "createaccountwithseed" => {
                    Some(ParsedSystemInstructionType::SystemCreateAccount)
                }
                _ => None,
            };

            if let Some(ix_type) = matching_type {
                if let Some(instructions) = parsed.get(&ix_type) {
                    let count = instructions
                        .iter()
                        .filter(|ix_data| {
                            match ix_data {
                                ParsedSystemInstructionData::SystemCreateAccount {
                                    payer, ..
                                } => {
                                    // Count instructions where Kora IS the payer
                                    // This tracks subsidized account creations
                                    kora_signer == Some(*payer)
                                }
                                _ => false,
                            }
                        })
                        .count() as u64;
                    counts[*idx] = count;
                } else {
                    counts[*idx] = 0;
                }
            }
        }
    }

    /// Batch count using manual parsing
    fn count_batch_manual(
        rules: &[(usize, &InstructionRule)],
        ctx: &LimiterContext<'_>,
        counts: &mut [u64],
    ) {
        for instruction in ctx.transaction.all_instructions.iter() {
            for (idx, rule) in rules {
                if instruction.program_id != rule.program {
                    continue;
                }

                if let Some(instr_type) =
                    InstructionIdentifier::identify(&instruction.program_id, &instruction.data)
                {
                    if instr_type == rule.instruction {
                        counts[*idx] += 1;
                    }
                }
            }
        }
    }

    pub fn storage_key(&self, user_id: &str, timestamp: u64) -> String {
        let base = format!("{IX_KEY_PREFIX}:{user_id}:{}:{}", self.program, self.instruction);
        match self.window_seconds {
            Some(window) if window > 0 => format!("{base}:{}", timestamp / window),
            _ => base,
        }
    }

    /// How many units to increment for this transaction
    pub fn count_increment(&self, ctx: &mut LimiterContext<'_>) -> u64 {
        Self::count_all_rules(&[self], ctx).into_iter().next().unwrap_or(0)
    }

    /// Maximum allowed count within the window (or lifetime)
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Time window in seconds
    pub fn window_seconds(&self) -> Option<u64> {
        self.window_seconds
    }

    pub fn description(&self) -> String {
        let window = self.window_seconds.map_or("lifetime".to_string(), |w| format!("per {w}s"));
        format!("{} on {} ({window})", self.instruction, self.program)
    }
}

pub struct InstructionIdentifier;

impl InstructionIdentifier {
    pub fn identify(program_id: &Pubkey, data: &[u8]) -> Option<String> {
        match *program_id {
            _ if *program_id == SYSTEM_PROGRAM_ID => Self::system(data),
            _ if *program_id == ATA_PROGRAM_ID => Self::ata(data),
            _ => None,
        }
    }

    fn system(data: &[u8]) -> Option<String> {
        let discriminator = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
        match discriminator {
            0 => Some("createaccount".to_string()),
            3 => Some("createaccountwithseed".to_string()),
            _ => None,
        }
    }

    fn ata(data: &[u8]) -> Option<String> {
        match data.first().copied() {
            None | Some(0) => Some("create".to_string()),
            Some(1) => Some("createidempotent".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::transaction_mock::create_mock_resolved_transaction;

    #[test]
    fn test_instruction_rule_lifetime_key() {
        let rule = InstructionRule::lifetime(SYSTEM_PROGRAM_ID, "createaccount".to_string(), 10);
        let user_id = "test-user-123";

        let key = rule.storage_key(user_id, 1000000);
        assert_eq!(key, format!("kora:ix:{}:{}:createaccount", user_id, SYSTEM_PROGRAM_ID));
    }

    #[test]
    fn test_instruction_rule_windowed_key() {
        let rule =
            InstructionRule::windowed(SYSTEM_PROGRAM_ID, "createaccount".to_string(), 10, 3600);
        let user_id = "test-user-456";

        let key1 = rule.storage_key(user_id, 3600);
        let key2 = rule.storage_key(user_id, 7199);
        let key3 = rule.storage_key(user_id, 7200);

        assert!(key1.ends_with(":1"));
        assert!(key2.ends_with(":1"));
        assert!(key3.ends_with(":2"));
    }

    #[test]
    fn test_instruction_rule_count_no_match() {
        let rule = InstructionRule::lifetime(SYSTEM_PROGRAM_ID, "createaccount".to_string(), 10);
        let tx = create_mock_resolved_transaction();
        let user_id = "test-user-789".to_string();
        let mut tx = tx;
        let mut ctx =
            LimiterContext { transaction: &mut tx, user_id, kora_signer: None, timestamp: 1000000 };

        assert_eq!(rule.count_increment(&mut ctx), 0);
    }

    #[test]
    fn test_instruction_rule_description() {
        let lifetime =
            InstructionRule::lifetime(SYSTEM_PROGRAM_ID, "createaccount".to_string(), 10);
        assert!(lifetime.description().contains("createaccount"));
        assert!(lifetime.description().contains("lifetime"));

        let windowed =
            InstructionRule::windowed(ATA_PROGRAM_ID, "createidempotent".to_string(), 5, 86400);
        assert!(windowed.description().contains("createidempotent"));
        assert!(windowed.description().contains("per 86400s"));
    }

    #[test]
    fn test_instruction_case_insensitive() {
        let rule = InstructionRule::new(SYSTEM_PROGRAM_ID, "CreateAccount".to_string(), 10, None);
        assert_eq!(rule.instruction, "createaccount");
    }

    #[test]
    fn test_identify_system_instructions() {
        assert_eq!(InstructionIdentifier::system(&[0, 0, 0, 0]), Some("createaccount".to_string()));
        assert_eq!(
            InstructionIdentifier::system(&[3, 0, 0, 0]),
            Some("createaccountwithseed".to_string())
        );
    }

    #[test]
    fn test_identify_ata_instructions() {
        assert_eq!(InstructionIdentifier::ata(&[]), Some("create".to_string()));
        assert_eq!(InstructionIdentifier::ata(&[0]), Some("create".to_string()));
        assert_eq!(InstructionIdentifier::ata(&[1]), Some("createidempotent".to_string()));
    }

    #[test]
    fn test_batch_counting_empty_rules() {
        let tx = create_mock_resolved_transaction();
        let user_id = "test-user-batch".to_string();
        let mut tx_mut = tx;
        let mut ctx = LimiterContext {
            transaction: &mut tx_mut,
            user_id,
            kora_signer: None,
            timestamp: 1000000,
        };

        let rules: Vec<&InstructionRule> = vec![];
        let counts = InstructionRule::count_all_rules(&rules, &mut ctx);
        assert_eq!(counts.len(), 0);
    }

    #[test]
    fn test_batch_counting_matches_individual() {
        let tx1 = create_mock_resolved_transaction();
        let tx2 = create_mock_resolved_transaction();
        let tx_batch = create_mock_resolved_transaction();
        let user_id = "test-user-individual".to_string();

        let rule1 = InstructionRule::lifetime(SYSTEM_PROGRAM_ID, "createaccount".to_string(), 10);
        let rule2 = InstructionRule::lifetime(ATA_PROGRAM_ID, "createidempotent".to_string(), 5);

        // Count individually
        let mut tx1_mut = tx1;
        let mut ctx1 = LimiterContext {
            transaction: &mut tx1_mut,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        let mut tx2_mut = tx2;
        let mut ctx2 = LimiterContext {
            transaction: &mut tx2_mut,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        let count1 = rule1.count_increment(&mut ctx1);
        let count2 = rule2.count_increment(&mut ctx2);

        // Count using batch method
        let mut tx_batch_mut = tx_batch;
        let mut ctx_batch = LimiterContext {
            transaction: &mut tx_batch_mut,
            user_id,
            kora_signer: None,
            timestamp: 1000000,
        };
        let rules = vec![&rule1, &rule2];
        let batch_counts = InstructionRule::count_all_rules(&rules, &mut ctx_batch);

        assert_eq!(batch_counts.len(), 2);
        assert_eq!(batch_counts[0], count1);
        assert_eq!(batch_counts[1], count2);
    }
}
