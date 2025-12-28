//! Instruction context tracking for CPI validation.
//!
//! Kora fetches inner instructions via simulation, but doesn't track which
//! outer instruction triggered each inner instruction. This module provides
//! that context for CPI-based fee validation.
//!
//! # How CPI Tracking Works
//!
//! When simulating a transaction with `inner_instructions: true`, the RPC returns
//! inner instructions grouped by the outer instruction that triggered them:
//!
//! ```text
//! Transaction:
//!   [0] Outer instruction (Program A)
//!       ├── [0.0] Inner instruction (CPI to Program B)
//!       └── [0.1] Inner instruction (CPI to Token Program)
//!   [1] Outer instruction (Program C)
//!       └── [1.0] Inner instruction (CPI to Token Program)
//! ```
//!
//! Each `UiInnerInstructions` has an `index` field matching the outer instruction.
//! We use this to identify which program initiated each token transfer CPI.

use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use solana_transaction_status_client_types::UiInnerInstructions;

/// Origin of an instruction in a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionOrigin {
    /// Top-level instruction in the transaction.
    TopLevel,

    /// CPI (Cross-Program Invocation) from another program.
    Cpi {
        /// The program that initiated the CPI.
        parent_program_id: Pubkey,
        /// Index of the outer instruction that triggered this CPI.
        outer_instruction_index: usize,
        /// Depth in the call stack (1 = direct CPI, 2 = nested CPI, etc.)
        depth: u8,
    },
}

impl InstructionOrigin {
    /// Check if this instruction is a CPI.
    pub fn is_cpi(&self) -> bool {
        matches!(self, InstructionOrigin::Cpi { .. })
    }

    /// Get the parent program ID if this is a CPI.
    pub fn parent_program_id(&self) -> Option<&Pubkey> {
        match self {
            InstructionOrigin::Cpi {
                parent_program_id, ..
            } => Some(parent_program_id),
            InstructionOrigin::TopLevel => None,
        }
    }
}

/// Instruction with its origin context.
#[derive(Debug, Clone)]
pub struct InstructionContext {
    /// The instruction itself.
    pub instruction: Instruction,

    /// Where this instruction came from.
    pub origin: InstructionOrigin,
}

impl InstructionContext {
    /// Create context for a top-level instruction.
    pub fn top_level(instruction: Instruction) -> Self {
        Self {
            instruction,
            origin: InstructionOrigin::TopLevel,
        }
    }

    /// Create context for a CPI instruction.
    pub fn cpi(
        instruction: Instruction,
        parent_program_id: Pubkey,
        outer_instruction_index: usize,
        depth: u8,
    ) -> Self {
        Self {
            instruction,
            origin: InstructionOrigin::Cpi {
                parent_program_id,
                outer_instruction_index,
                depth,
            },
        }
    }

    /// Check if this instruction is a CPI.
    pub fn is_cpi(&self) -> bool {
        self.origin.is_cpi()
    }

    /// Get the parent program ID if this is a CPI.
    pub fn parent_program_id(&self) -> Option<&Pubkey> {
        self.origin.parent_program_id()
    }
}

/// Maps inner instruction indices to their parent outer instructions.
///
/// This struct helps track which outer instruction triggered each group
/// of inner instructions from simulation results.
#[derive(Debug, Default)]
pub struct InnerInstructionMap {
    /// Maps outer instruction index to the program ID of that instruction.
    outer_programs: Vec<Pubkey>,
}

impl InnerInstructionMap {
    /// Build a map from outer instructions.
    pub fn from_outer_instructions(instructions: &[Instruction]) -> Self {
        Self {
            outer_programs: instructions.iter().map(|ix| ix.program_id).collect(),
        }
    }

    /// Get the parent program for an inner instruction group.
    ///
    /// The `outer_index` comes from `UiInnerInstructions.index`.
    pub fn get_parent_program(&self, outer_index: usize) -> Option<Pubkey> {
        self.outer_programs.get(outer_index).copied()
    }

    /// Check if an outer instruction is from an allowed program.
    pub fn is_from_allowed_program(&self, outer_index: usize, allowed_programs: &[Pubkey]) -> bool {
        self.get_parent_program(outer_index)
            .map(|program| allowed_programs.contains(&program))
            .unwrap_or(false)
    }
}

/// Extract the outer instruction index from `UiInnerInstructions`.
///
/// This is a helper to handle the type from transaction simulation results.
pub fn get_outer_instruction_index(inner_instructions: &UiInnerInstructions) -> usize {
    inner_instructions.index as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_instruction(program_id: Pubkey) -> Instruction {
        Instruction::new_with_bytes(program_id, &[1, 2, 3], vec![])
    }

    #[test]
    fn test_instruction_origin_top_level() {
        let origin = InstructionOrigin::TopLevel;
        assert!(!origin.is_cpi());
        assert!(origin.parent_program_id().is_none());
    }

    #[test]
    fn test_instruction_origin_cpi() {
        let parent = Pubkey::new_unique();
        let origin = InstructionOrigin::Cpi {
            parent_program_id: parent,
            outer_instruction_index: 0,
            depth: 1,
        };

        assert!(origin.is_cpi());
        assert_eq!(origin.parent_program_id(), Some(&parent));
    }

    #[test]
    fn test_instruction_context_top_level() {
        let program_id = Pubkey::new_unique();
        let instruction = make_test_instruction(program_id);
        let ctx = InstructionContext::top_level(instruction);

        assert!(!ctx.is_cpi());
        assert!(ctx.parent_program_id().is_none());
        assert_eq!(ctx.instruction.program_id, program_id);
    }

    #[test]
    fn test_instruction_context_cpi() {
        let program_id = Pubkey::new_unique();
        let parent_program_id = Pubkey::new_unique();
        let instruction = make_test_instruction(program_id);

        let ctx = InstructionContext::cpi(instruction, parent_program_id, 0, 1);

        assert!(ctx.is_cpi());
        assert_eq!(ctx.parent_program_id(), Some(&parent_program_id));
        assert_eq!(ctx.instruction.program_id, program_id);
    }

    #[test]
    fn test_inner_instruction_map() {
        let program_a = Pubkey::new_unique();
        let program_b = Pubkey::new_unique();

        let outer_instructions = vec![
            make_test_instruction(program_a),
            make_test_instruction(program_b),
        ];

        let map = InnerInstructionMap::from_outer_instructions(&outer_instructions);

        assert_eq!(map.get_parent_program(0), Some(program_a));
        assert_eq!(map.get_parent_program(1), Some(program_b));
        assert_eq!(map.get_parent_program(2), None);
    }

    #[test]
    fn test_is_from_allowed_program() {
        let privacy_program = Pubkey::new_unique();
        let other_program = Pubkey::new_unique();
        let allowed = vec![privacy_program];

        let outer_instructions = vec![
            make_test_instruction(privacy_program),
            make_test_instruction(other_program),
        ];

        let map = InnerInstructionMap::from_outer_instructions(&outer_instructions);

        // Index 0 is privacy program - should be allowed
        assert!(map.is_from_allowed_program(0, &allowed));

        // Index 1 is other program - should not be allowed
        assert!(!map.is_from_allowed_program(1, &allowed));

        // Out of bounds - should not be allowed
        assert!(!map.is_from_allowed_program(99, &allowed));
    }
}
