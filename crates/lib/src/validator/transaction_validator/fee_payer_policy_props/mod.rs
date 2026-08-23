mod alt;
mod spl_token;
mod system;

use super::*;
use crate::{
    config::FeePayerPolicy, tests::config_mock::ConfigMockBuilder, transaction::TransactionUtil,
};
use proptest::{prelude::*, test_runner::TestCaseError};
use solana_message::{Message, VersionedMessage};
use solana_sdk::instruction::Instruction;

// One gated fee-payer role: an instruction the fee payer can appear in, and the policy flag
// that gates it. `flag` returns a mutable reference so the property sets and reads the same
// field by construction, which is what stops a mis-mapped role from passing vacuously.
trait DrainRole: Copy + std::fmt::Debug + 'static {
    const ROLES: &'static [Self];

    fn allowed_programs() -> Vec<String>;
    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool;
    fn instruction(self, actor: &Pubkey) -> Instruction;
}

fn role_and_flags<R: DrainRole>() -> impl Strategy<Value = (usize, bool, Vec<bool>)> {
    (0..R::ROLES.len(), any::<bool>(), prop::collection::vec(any::<bool>(), R::ROLES.len()))
}

fn validate<R: DrainRole>(
    policy: FeePayerPolicy,
    fee_payer: Pubkey,
    ix: Instruction,
) -> Result<(), KoraError> {
    let config = ConfigMockBuilder::new()
        .with_price_source(PriceSource::Mock)
        .with_allowed_programs(R::allowed_programs())
        .with_max_allowed_lamports(1_000_000)
        // AdvanceNonceAccount is rejected before the policy gate unless durable txs are
        // allowed; enable so the nonce policy flags are what's under test.
        .with_allow_durable_transactions(true)
        .with_fee_payer_policy(policy)
        .build();
    let validator = TransactionValidator::new(&config, fee_payer).unwrap();
    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
    let mut resolved =
        TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
    validator.validate_fee_payer_usage(&config, &mut resolved)
}

fn assert_role_gated_iff_flag_off<R: DrainRole>(
    role_idx: usize,
    actor_is_fee_payer: bool,
    flags: &[bool],
) -> Result<(), TestCaseError> {
    let role = R::ROLES[role_idx];

    let mut policy = FeePayerPolicy::default();
    for (other, allowed) in R::ROLES.iter().zip(flags) {
        *other.flag(&mut policy) = *allowed;
    }
    let flag = *role.flag(&mut policy);

    let fee_payer = Pubkey::new_unique();
    let actor = if actor_is_fee_payer { fee_payer } else { Pubkey::new_unique() };
    let result = validate::<R>(policy, fee_payer, role.instruction(&actor));

    if !actor_is_fee_payer {
        prop_assert!(
            result.is_ok(),
            "{role:?} by a non-fee-payer must never be gated, got {result:?}"
        );
    } else if flag {
        prop_assert!(
            result.is_ok(),
            "{role:?} by fee payer must pass when its flag is on, got {result:?}"
        );
    } else {
        prop_assert!(
            result.is_err(),
            "{role:?} by fee payer must be rejected when its flag is off"
        );
    }

    Ok(())
}
