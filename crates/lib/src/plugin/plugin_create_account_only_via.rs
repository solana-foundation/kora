use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::{collections::HashSet, str::FromStr};

use crate::{
    config::Config,
    error::KoraError,
    token::token::TokenUtil,
    transaction::{
        InstructionOrigin, IxUtils, ParsedSystemInstructionData, VersionedTransactionResolved,
    },
    validator::parse_pubkey_set,
};

use super::{PluginExecutionContext, TransactionPlugin};

pub(super) struct CreateAccountOnlyViaPlugin;

impl CreateAccountOnlyViaPlugin {
    fn validate_creations(
        transaction: &VersionedTransactionResolved,
        programs: &HashSet<Pubkey>,
        fee_payer: &Pubkey,
    ) -> Result<(), KoraError> {
        let all_instructions = &transaction.all_instructions;
        let origins = &transaction.instruction_origins;
        if all_instructions.len() != origins.len() {
            return Err(KoraError::InvalidTransaction(format!(
                "Plugin create_account_only_via resolved {} instructions but {} instruction origins",
                all_instructions.len(),
                origins.len()
            )));
        }
        for (index, (instruction, origin)) in all_instructions.iter().zip(origins).enumerate() {
            let Some(creation) = Self::fee_payer_funded_creation(instruction, fee_payer)? else {
                continue;
            };
            let issued_by = match origin {
                InstructionOrigin::TopLevel => None,
                InstructionOrigin::Inner { parent_index } => {
                    Some(all_instructions[*parent_index].program_id)
                }
            };
            if issued_by.is_some_and(|program| programs.contains(&program)) {
                continue;
            }
            let location = match issued_by {
                None => "a top-level instruction".to_string(),
                Some(program) => format!("a CPI from program {program}"),
            };
            return Err(KoraError::InvalidTransaction(format!(
                "Plugin create_account_only_via lets the fee payer fund account creation only \
                 inside a CPI from a listed program; instruction {index} ('{creation}') is {location}"
            )));
        }

        Ok(())
    }

    /// Names the account creation `instruction` performs with the fee payer as funder, if any.
    fn fee_payer_funded_creation(
        instruction: &Instruction,
        fee_payer: &Pubkey,
    ) -> Result<Option<&'static str>, KoraError> {
        if let Some(ParsedSystemInstructionData::SystemCreateAccount { payer, .. }) =
            IxUtils::parse_system_instruction(instruction)?
        {
            return Ok((payer == *fee_payer).then_some("System Create Account"));
        }
        Ok(TokenUtil::parse_ata_creation_instruction(instruction)
            .filter(|ata| ata.payer == *fee_payer)
            .map(|_| "Associated Token Account Create"))
    }
}

#[async_trait]
impl TransactionPlugin for CreateAccountOnlyViaPlugin {
    async fn validate(
        &self,
        transaction: &VersionedTransactionResolved,
        config: &Config,
        _rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        _context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let programs = parse_pubkey_set(&config.kora.plugins.create_account_only_via.programs)?;
        Self::validate_creations(transaction, &programs, fee_payer)
    }

    fn validate_config(&self, config: &Config) -> (Vec<String>, Vec<String>) {
        let programs = &config.kora.plugins.create_account_only_via.programs;
        let mut errors = Vec::new();
        if programs.is_empty() {
            errors.push(
                "CreateAccountOnlyVia plugin requires at least one program in \
                 [kora.plugins.create_account_only_via] programs"
                    .to_string(),
            );
        }
        for program in programs {
            if Pubkey::from_str(program).is_err() {
                errors.push(format!(
                    "Invalid base58 pubkey format in [kora.plugins.create_account_only_via] programs: {program}"
                ));
            } else if !config.validation.allowed_programs.contains(program) {
                errors.push(format!(
                    "Program {program} in [kora.plugins.create_account_only_via] programs must also \
                     be in allowed_programs"
                ));
            }
        }

        let mut warnings = Vec::new();
        if !config.validation.fee_payer_policy.system.allow_create_account {
            warnings.push(
                "CreateAccountOnlyVia plugin has no effect while \
                 [validation.fee_payer_policy.system] allow_create_account=false: the fee payer \
                 cannot fund any account creation"
                    .to_string(),
            );
        }

        (errors, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::TransactionPluginRunner, *};
    use crate::{
        config::{FeePayerPolicy, TransactionPluginType},
        constant::instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR,
        oracle::PriceSource,
        tests::{common::RpcMockBuilder, config_mock::ConfigMockBuilder},
        transaction::TransactionUtil,
        validator::transaction_validator::TransactionValidator,
    };
    use proptest::prelude::*;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{instruction::AccountMeta, transaction::VersionedTransaction};
    use solana_system_interface::{
        instruction::{create_account, create_account_with_seed},
        program::ID as SYSTEM_PROGRAM_ID,
    };
    use spl_associated_token_account_interface::instruction::{
        create_associated_token_account, create_associated_token_account_idempotent,
    };
    use std::sync::Arc;

    struct Fixture {
        config: Config,
        fee_payer: Pubkey,
        listed_program: Pubkey,
        unlisted_program: Pubkey,
        rpc_client: Arc<RpcClient>,
    }

    impl Fixture {
        fn setup(plugin_enabled: bool) -> Self {
            let fee_payer = Pubkey::new_unique();
            let listed_program = Pubkey::new_unique();
            let unlisted_program = Pubkey::new_unique();
            let mut policy = FeePayerPolicy::default();
            policy.system.allow_create_account = true;
            let mut config = ConfigMockBuilder::new()
                .with_price_source(PriceSource::Mock)
                .with_allowed_programs(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    spl_associated_token_account_interface::program::id().to_string(),
                    listed_program.to_string(),
                    unlisted_program.to_string(),
                ])
                .with_max_allowed_lamports(10_000_000)
                .with_fee_payer_policy(policy)
                .build();
            if plugin_enabled {
                config.kora.plugins.enabled = vec![TransactionPluginType::CreateAccountOnlyVia];
                config.kora.plugins.create_account_only_via.programs =
                    vec![listed_program.to_string()];
            }
            let rpc_client = RpcMockBuilder::new()
                .with_custom_mock(
                    solana_client::rpc_request::RpcRequest::GetMinimumBalanceForRentExemption,
                    serde_json::json!(2_039_280),
                )
                .build();
            Self { config, fee_payer, listed_program, unlisted_program, rpc_client }
        }

        fn call(&self, program: Pubkey) -> Instruction {
            Instruction { program_id: program, accounts: vec![], data: vec![] }
        }

        fn fee_payer_creates_account(&self) -> Instruction {
            create_account(&self.fee_payer, &Pubkey::new_unique(), 1_000, 8, &SYSTEM_PROGRAM_ID)
        }

        fn fee_payer_creates_ata(&self) -> Instruction {
            create_associated_token_account(
                &self.fee_payer,
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &spl_token_interface::id(),
            )
        }

        /// Runs the validator and then the plugins, as the sign flow does.
        async fn validate(
            &self,
            transaction: &mut VersionedTransactionResolved,
        ) -> Result<(), KoraError> {
            TransactionValidator::new(&self.config, self.fee_payer)?
                .validate_transaction(&self.config, transaction, &self.rpc_client)
                .await?;
            TransactionPluginRunner::from_config(&self.config)
                .run(
                    transaction,
                    &self.config,
                    &self.rpc_client,
                    &self.fee_payer,
                    PluginExecutionContext::SignTransaction,
                )
                .await
        }

        /// Validates `top_level` plus `inner` CPIs, each attributed to the top-level index given.
        async fn validate_with(
            &self,
            top_level: &[Instruction],
            inner: &[(usize, Instruction)],
        ) -> Result<(), KoraError> {
            let message = VersionedMessage::Legacy(Message::new(top_level, Some(&self.fee_payer)));
            let mut transaction =
                TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
            for (parent_index, instruction) in inner {
                transaction.push_inner_instruction(*parent_index, instruction.clone()).unwrap();
            }
            self.validate(&mut transaction).await
        }
    }

    fn assert_only_via_rejection(result: Result<(), KoraError>, location: &str) {
        let err = result.expect_err("fee payer funded creation outside the listed CPI");
        let msg = err.to_string();
        assert!(
            msg.contains("Plugin create_account_only_via lets the fee payer fund account creation")
                && msg.contains(location),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    async fn only_via_rejects_top_level_fee_payer_create_account() {
        let fixture = Fixture::setup(true);

        assert_only_via_rejection(
            fixture.validate_with(&[fixture.fee_payer_creates_account()], &[]).await,
            "instruction 0 ('System Create Account') is a top-level instruction",
        );
    }

    #[tokio::test]
    async fn only_via_accepts_create_account_inside_listed_program_cpi() {
        let fixture = Fixture::setup(true);

        fixture
            .validate_with(
                &[fixture.call(fixture.listed_program)],
                &[(0, fixture.fee_payer_creates_account())],
            )
            .await
            .expect("CPI from the listed program may fund account creation");
    }

    #[tokio::test]
    async fn only_via_accepts_nested_ata_and_system_create_under_listed_program() {
        let fixture = Fixture::setup(true);

        fixture
            .validate_with(
                &[fixture.call(fixture.listed_program)],
                &[(0, fixture.fee_payer_creates_ata()), (0, fixture.fee_payer_creates_account())],
            )
            .await
            .expect("nested CPIs under the listed program may fund account creation");
    }

    #[tokio::test]
    async fn only_via_rejects_top_level_fee_payer_ata_create() {
        let fixture = Fixture::setup(true);

        assert_only_via_rejection(
            fixture
                .validate_with(
                    &[fixture.call(fixture.listed_program), fixture.fee_payer_creates_ata()],
                    &[],
                )
                .await,
            "instruction 1 ('Associated Token Account Create') is a top-level instruction",
        );
    }

    #[tokio::test]
    async fn only_via_rejects_sibling_top_level_create_when_program_called() {
        let fixture = Fixture::setup(true);

        assert_only_via_rejection(
            fixture
                .validate_with(
                    &[fixture.call(fixture.listed_program), fixture.fee_payer_creates_account()],
                    &[],
                )
                .await,
            "instruction 1 ('System Create Account') is a top-level instruction",
        );
    }

    #[tokio::test]
    async fn only_via_rejects_create_account_inside_unlisted_program_cpi() {
        let fixture = Fixture::setup(true);

        assert_only_via_rejection(
            fixture
                .validate_with(
                    &[fixture.call(fixture.listed_program), fixture.call(fixture.unlisted_program)],
                    &[(1, fixture.fee_payer_creates_account())],
                )
                .await,
            &format!("is a CPI from program {}", fixture.unlisted_program),
        );
    }

    #[tokio::test]
    async fn only_via_rejects_parsed_ata_cpi_from_simulation() {
        let fixture = Fixture::setup(true);
        let ata_instruction = create_associated_token_account_idempotent(
            &fixture.fee_payer,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &spl_token_interface::id(),
        );
        let ata_message = Message::new(&[ata_instruction.clone()], None);
        let parsed_ata = solana_transaction_status::parse_instruction::parse(
            &ata_instruction.program_id,
            &ata_message.instructions[0],
            &solana_message::AccountKeys::new(&ata_message.account_keys, None),
            Some(2),
        )
        .unwrap();

        let mut unlisted_call = fixture.call(fixture.unlisted_program);
        unlisted_call.accounts = ata_instruction.accounts.clone();
        unlisted_call.accounts.push(AccountMeta::new_readonly(ata_instruction.program_id, false));
        let message = VersionedMessage::Legacy(Message::new(
            &[fixture.call(fixture.listed_program), unlisted_call],
            Some(&fixture.fee_payer),
        ));
        let transaction = VersionedTransaction {
            signatures: vec![Default::default(); message.header().num_required_signatures as usize],
            message,
        };

        let simulation_rpc = RpcMockBuilder::new()
            .with_custom_mock(
                solana_client::rpc_request::RpcRequest::SimulateTransaction,
                serde_json::json!({
                    "context": { "slot": 1 },
                    "value": {
                        "err": null,
                        "logs": [],
                        "accounts": null,
                        "unitsConsumed": 1000,
                        "innerInstructions": [{
                            "index": 1,
                            "instructions": [
                                solana_transaction_status_client_types::UiInstruction::Parsed(
                                    solana_transaction_status_client_types::UiParsedInstruction::Parsed(
                                        parsed_ata,
                                    ),
                                ),
                            ],
                        }],
                    },
                }),
            )
            .build();
        let mut resolved = VersionedTransactionResolved::from_transaction(
            &transaction,
            &fixture.config,
            &simulation_rpc,
            false,
            None,
        )
        .await
        .unwrap();

        assert_only_via_rejection(
            fixture.validate(&mut resolved).await,
            &format!(
                "instruction 2 ('Associated Token Account Create') is a CPI from program {}",
                fixture.unlisted_program
            ),
        );
    }

    #[tokio::test]
    async fn only_via_ignores_creations_funded_by_others() {
        let fixture = Fixture::setup(true);

        fixture
            .validate_with(
                &[create_account(
                    &Pubkey::new_unique(),
                    &Pubkey::new_unique(),
                    1_000,
                    8,
                    &SYSTEM_PROGRAM_ID,
                )],
                &[],
            )
            .await
            .expect("a creation the fee payer does not fund is outside the rule");
    }

    #[tokio::test]
    async fn only_via_rejects_instruction_without_origin() {
        let fixture = Fixture::setup(true);
        let message = VersionedMessage::Legacy(Message::new(
            &[fixture.call(fixture.listed_program)],
            Some(&fixture.fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        transaction.all_instructions.push(fixture.fee_payer_creates_account());

        let err = fixture
            .validate(&mut transaction)
            .await
            .expect_err("an instruction without an origin must not pass the rule");
        assert!(
            err.to_string().contains("resolved 2 instructions but 1 instruction origins"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn only_via_disabled_plugin_keeps_top_level_create_allowed() {
        let fixture = Fixture::setup(false);

        fixture
            .validate_with(
                &[fixture.fee_payer_creates_account(), fixture.fee_payer_creates_ata()],
                &[],
            )
            .await
            .expect("without the plugin, allow_create_account alone decides");
    }

    #[test]
    fn only_via_validate_config_reports_errors_and_warnings() {
        let listed = SYSTEM_PROGRAM_ID.to_string();
        let unlisted = Pubkey::new_unique().to_string();
        let cases: [(Vec<String>, bool, &str); 4] = [
            (vec![], true, "requires at least one program"),
            (vec!["not-a-pubkey".to_string()], true, "Invalid base58 pubkey format"),
            (vec![unlisted.clone()], true, "must also be in allowed_programs"),
            (vec![listed.clone()], false, "has no effect while"),
        ];
        for (programs, allow_create_account, expected) in cases {
            let mut config =
                ConfigMockBuilder::new().with_allowed_programs(vec![listed.clone()]).build();
            config.validation.fee_payer_policy.system.allow_create_account = allow_create_account;
            config.kora.plugins.create_account_only_via.programs = programs.clone();

            let (errors, warnings) = CreateAccountOnlyViaPlugin.validate_config(&config);

            assert!(
                errors.iter().chain(&warnings).any(|m| m.contains(expected)),
                "{programs:?}, allow_create_account={allow_create_account}: {errors:?} {warnings:?}"
            );
        }

        let mut config =
            ConfigMockBuilder::new().with_allowed_programs(vec![listed.clone()]).build();
        config.validation.fee_payer_policy.system.allow_create_account = true;
        config.kora.plugins.create_account_only_via.programs = vec![listed];
        assert_eq!(CreateAccountOnlyViaPlugin.validate_config(&config), (vec![], vec![]));
    }

    #[derive(Debug, Clone, Copy)]
    enum Creation {
        CreateAccount,
        CreateAccountWithSeed,
        CreateAccountAllowPrefund,
        AtaCreate,
        AtaCreateIdempotent,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Placement {
        TopLevel,
        InnerUnderListed,
        InnerUnderUnlisted,
    }

    const CREATIONS: &[Creation] = &[
        Creation::CreateAccount,
        Creation::CreateAccountWithSeed,
        Creation::CreateAccountAllowPrefund,
        Creation::AtaCreate,
        Creation::AtaCreateIdempotent,
    ];

    const PLACEMENTS: &[Placement] =
        &[Placement::TopLevel, Placement::InnerUnderListed, Placement::InnerUnderUnlisted];

    fn creation_funded_by(kind: Creation, funder: &Pubkey) -> Instruction {
        match kind {
            Creation::CreateAccount => {
                create_account(funder, &Pubkey::new_unique(), 1_000, 8, &SYSTEM_PROGRAM_ID)
            }
            Creation::CreateAccountWithSeed => create_account_with_seed(
                funder,
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                "seed",
                1_000,
                8,
                &SYSTEM_PROGRAM_ID,
            ),
            Creation::CreateAccountAllowPrefund => Instruction {
                program_id: SYSTEM_PROGRAM_ID,
                accounts: vec![
                    AccountMeta::new(Pubkey::new_unique(), true),
                    AccountMeta::new(*funder, true),
                ],
                data: bincode::serialize(&(DISCRIMINATOR, 1_000u64, 8u64, SYSTEM_PROGRAM_ID))
                    .unwrap(),
            },
            Creation::AtaCreate => create_associated_token_account(
                funder,
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &spl_token_interface::id(),
            ),
            Creation::AtaCreateIdempotent => create_associated_token_account_idempotent(
                funder,
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &spl_token_interface::id(),
            ),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn only_via_rejects_exactly_fee_payer_funded_creations_outside_listed_cpis(
            kind in prop::sample::select(CREATIONS),
            placement in prop::sample::select(PLACEMENTS),
            funder_is_fee_payer in any::<bool>(),
        ) {
            let fee_payer = Pubkey::new_unique();
            let listed = Pubkey::new_unique();
            let unlisted = Pubkey::new_unique();
            let funder = if funder_is_fee_payer { fee_payer } else { Pubkey::new_unique() };

            let call = |program: Pubkey| Instruction { program_id: program, accounts: vec![], data: vec![] };
            let creation = creation_funded_by(kind, &funder);
            let mut top_level = vec![call(listed), call(unlisted)];
            let parent_index = match placement {
                Placement::TopLevel => {
                    top_level.push(creation.clone());
                    None
                }
                Placement::InnerUnderListed => Some(0),
                Placement::InnerUnderUnlisted => Some(1),
            };
            let message = VersionedMessage::Legacy(Message::new(&top_level, Some(&fee_payer)));
            let mut resolved =
                TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
            if let Some(parent_index) = parent_index {
                resolved.push_inner_instruction(parent_index, creation).unwrap();
            }

            let result = CreateAccountOnlyViaPlugin::validate_creations(
                &resolved,
                &HashSet::from([listed]),
                &fee_payer,
            );
            let expected_ok = !funder_is_fee_payer || placement == Placement::InnerUnderListed;
            prop_assert_eq!(
                result.is_ok(),
                expected_ok,
                "{:?} at {:?}, funder_is_fee_payer={}: {:?}",
                kind, placement, funder_is_fee_payer, result
            );
        }
    }
}
