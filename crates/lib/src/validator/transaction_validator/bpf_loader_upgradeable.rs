use super::TransactionValidator;
use crate::{
    error::KoraError,
    transaction::{
        ParsedBpfLoaderUpgradeableInstructionData, ParsedBpfLoaderUpgradeableInstructionType,
    },
};
use std::collections::HashMap;

impl TransactionValidator {
    pub(super) fn validate_bpf_loader_upgradeable_fee_payer_usage(
        &self,
        bpf_v3_instructions: &HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        >,
    ) -> Result<(), KoraError> {
        // InitializeBuffer: authority is optional. Only gated when present and == fee_payer.
        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::InitializeBuffer,
            ParsedBpfLoaderUpgradeableInstructionData::InitializeBuffer { authority, .. } =>
            authority.is_some_and(|a| a == self.fee_payer_pubkey),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_initialize_buffer,
            "Fee payer cannot be used for 'BPF Loader Upgradeable InitializeBuffer'");

        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::Write,
            ParsedBpfLoaderUpgradeableInstructionData::Write { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_write,
            "Fee payer cannot be used for 'BPF Loader Upgradeable Write'");

        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::DeployWithMaxDataLen,
            ParsedBpfLoaderUpgradeableInstructionData::DeployWithMaxDataLen {
                payer, upgrade_authority, ..
            } => (*payer == self.fee_payer_pubkey
                || *upgrade_authority == self.fee_payer_pubkey),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_deploy_with_max_data_len,
            "Fee payer cannot be used for 'BPF Loader Upgradeable DeployWithMaxDataLen'");

        // Gate only on the upgrade_authority signing role. `spill` is a lamport recipient,
        // not a signer — a user upgrading their own program can refund excess lamports to
        // Kora without that being a Kora-as-authority operation.
        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::Upgrade,
            ParsedBpfLoaderUpgradeableInstructionData::Upgrade { upgrade_authority, .. } =>
            *upgrade_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_upgrade,
            "Fee payer cannot be used for 'BPF Loader Upgradeable Upgrade'");

        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::SetAuthority,
            ParsedBpfLoaderUpgradeableInstructionData::SetAuthority {
                current_authority, new_authority, ..
            } => (*current_authority == self.fee_payer_pubkey
                || new_authority.is_some_and(|n| n == self.fee_payer_pubkey)),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_set_authority,
            "Fee payer cannot be used for 'BPF Loader Upgradeable SetAuthority'");

        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::SetAuthorityChecked,
            ParsedBpfLoaderUpgradeableInstructionData::SetAuthorityChecked {
                current_authority, new_authority, ..
            } => (*current_authority == self.fee_payer_pubkey
                || *new_authority == self.fee_payer_pubkey),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_set_authority_checked,
            "Fee payer cannot be used for 'BPF Loader Upgradeable SetAuthorityChecked'");

        // Gate only on the authority signing role. `recipient` is a lamport sink, not a
        // signer — a user closing their own buffer can legitimately refund lamports to Kora
        // without that being a Kora-as-authority operation. The drainage guard below still
        // catches the only real abuse vector: Kora as authority + foreign recipient.
        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::Close,
            ParsedBpfLoaderUpgradeableInstructionData::Close { authority, .. } =>
            authority.is_some_and(|a| a == self.fee_payer_pubkey),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_close,
            "Fee payer cannot be used for 'BPF Loader Upgradeable Close'");

        // Drainage guard for Close: when the fee payer is the authority, the recipient must
        // also be the fee payer. Otherwise the closed-account lamports flow to whoever the
        // attacker put as recipient.
        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::Close,
            ParsedBpfLoaderUpgradeableInstructionData::Close { authority, recipient, .. } =>
            authority.is_some_and(|a| a == self.fee_payer_pubkey)
                && *recipient != self.fee_payer_pubkey,
            "BPF Loader Upgradeable Close: when fee payer is the authority, \
             recipient must also be the fee payer (drainage guard)");

        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::ExtendProgram,
            ParsedBpfLoaderUpgradeableInstructionData::ExtendProgram { payer, .. } =>
            payer.is_some_and(|p| p == self.fee_payer_pubkey),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_extend_program,
            "Fee payer cannot be used for 'BPF Loader Upgradeable ExtendProgram'");

        // ExtendProgramChecked: like ExtendProgram but the authority is also a required
        // signer. Gate when fee_payer is either the authority or the (optional) payer.
        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::ExtendProgramChecked,
            ParsedBpfLoaderUpgradeableInstructionData::ExtendProgramChecked {
                authority, payer, ..
            } => (*authority == self.fee_payer_pubkey
                || payer.is_some_and(|p| p == self.fee_payer_pubkey)),
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_extend_program_checked,
            "Fee payer cannot be used for 'BPF Loader Upgradeable ExtendProgramChecked'");

        deny_fee_payer!(bpf_v3_instructions, ParsedBpfLoaderUpgradeableInstructionType::Migrate,
            ParsedBpfLoaderUpgradeableInstructionData::Migrate { current_authority, .. } =>
            *current_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.bpf_loader_upgradeable.allow_migrate,
            "Fee payer cannot be used for 'BPF Loader Upgradeable Migrate'");

        Ok(())
    }
}
