use super::ConfigValidator;
use crate::config::FeePayerPolicy;

impl ConfigValidator {
    /// Validate fee payer policy and add warnings for enabled risky operations
    pub(super) fn validate_fee_payer_policy(policy: &FeePayerPolicy, warnings: &mut Vec<String>) {
        macro_rules! check_fee_payer_policy {
        ($($category:ident, $field:ident, $description:expr, $risk:expr);* $(;)?) => {
            $(
                if policy.$category.$field {
                    warnings.push(format!(
                        "⚠️  SECURITY: Fee payer policy allows {} ({}). \
                        Risk: {}. \
                        Consider setting [validation.fee_payer_policy.{}] {}=false to prevent abuse.",
                        $description,
                        stringify!($field),
                        $risk,
                        stringify!($category),
                        stringify!($field)
                    ));
                }
            )*
        };
    }

        check_fee_payer_policy! {
            system, allow_transfer, "System transfers",
                "Users can make the fee payer transfer arbitrary SOL amounts. This can drain your fee payer account";

            system, allow_assign, "System Assign instructions",
                "Users can make the fee payer reassign ownership of its accounts. This can compromise account control";

            system, allow_allocate, "System Allocate instructions",
                "Users can make the fee payer allocate space for accounts. This can be used to waste resources";

            spl_token, allow_transfer, "SPL Token transfers",
                "Users can make the fee payer transfer arbitrary token amounts. This can drain your fee payer token accounts";

            spl_token, allow_burn, "SPL Token burn operations",
                "Users can make the fee payer burn tokens from its accounts. This causes permanent loss of assets";

            spl_token, allow_close_account, "SPL Token CloseAccount instructions",
                "Users can make the fee payer close token accounts. This can disrupt operations and drain fee payer";

            spl_token, allow_approve, "SPL Token approve operations",
                "Users can make the fee payer approve delegates. This can lead to unauthorized token transfers";

            spl_token, allow_revoke, "SPL Token revoke operations",
                "Users can make the fee payer revoke delegates. This can disrupt authorized operations";

            spl_token, allow_set_authority, "SPL Token SetAuthority instructions",
                "Users can make the fee payer transfer authority. This can lead to complete loss of control";

            spl_token, allow_mint_to, "SPL Token MintTo operations",
                "Users can make the fee payer mint tokens. This can inflate token supply";

            spl_token, allow_initialize_mint, "SPL Token InitializeMint instructions",
                "Users can make the fee payer initialize mints with itself as authority. This can lead to unexpected responsibilities";

            spl_token, allow_initialize_account, "SPL Token InitializeAccount instructions",
                "Users can make the fee payer the owner of new token accounts. This can clutter or exploit the fee payer";

            spl_token, allow_initialize_multisig, "SPL Token InitializeMultisig instructions",
                "Users can make the fee payer part of multisig accounts. This can create unwanted signing obligations";

            spl_token, allow_freeze_account, "SPL Token FreezeAccount instructions",
                "Users can make the fee payer freeze token accounts. This can disrupt token operations";

            spl_token, allow_thaw_account, "SPL Token ThawAccount instructions",
                "Users can make the fee payer unfreeze token accounts. This can undermine freeze policies";

            token_2022, allow_transfer, "Token2022 transfers",
                "Users can make the fee payer transfer arbitrary token amounts. This can drain your fee payer token accounts";

            token_2022, allow_burn, "Token2022 burn operations",
                "Users can make the fee payer burn tokens from its accounts. This causes permanent loss of assets";

            token_2022, allow_close_account, "Token2022 CloseAccount instructions",
                "Users can make the fee payer close token accounts. This can disrupt operations";

            token_2022, allow_approve, "Token2022 approve operations",
                "Users can make the fee payer approve delegates. This can lead to unauthorized token transfers";

            token_2022, allow_revoke, "Token2022 revoke operations",
                "Users can make the fee payer revoke delegates. This can disrupt authorized operations";

            token_2022, allow_set_authority, "Token2022 SetAuthority instructions",
                "Users can make the fee payer transfer authority. This can lead to complete loss of control";

            token_2022, allow_mint_to, "Token2022 MintTo operations",
                "Users can make the fee payer mint tokens. This can inflate token supply";

            token_2022, allow_initialize_mint, "Token2022 InitializeMint instructions",
                "Users can make the fee payer initialize mints with itself as authority. This can lead to unexpected responsibilities";

            token_2022, allow_initialize_account, "Token2022 InitializeAccount instructions",
                "Users can make the fee payer the owner of new token accounts. This can clutter or exploit the fee payer";

            token_2022, allow_initialize_multisig, "Token2022 InitializeMultisig instructions",
                "Users can make the fee payer part of multisig accounts. This can create unwanted signing obligations";

            token_2022, allow_freeze_account, "Token2022 FreezeAccount instructions",
                "Users can make the fee payer freeze token accounts. This can disrupt token operations";

            token_2022, allow_thaw_account, "Token2022 ThawAccount instructions",
                "Users can make the fee payer unfreeze token accounts. This can undermine freeze policies";

            alt, allow_create, "ALT CreateLookupTable instructions",
                "Users can make the fee payer create lookup tables and fund rent. This can drain SOL and create unmanaged tables";

            alt, allow_extend, "ALT ExtendLookupTable instructions",
                "Users can make the fee payer extend lookup tables and pay reallocation costs. This can drain SOL";

            alt, allow_freeze, "ALT FreezeLookupTable instructions",
                "Users can permanently freeze lookup tables controlled by the fee payer. This can break versioned transaction flows";

            alt, allow_deactivate, "ALT DeactivateLookupTable instructions",
                "Users can deactivate lookup tables controlled by the fee payer. This can disrupt systems that depend on those tables";

            alt, allow_close, "ALT CloseLookupTable instructions",
                "Users can close lookup tables controlled by the fee payer and redirect lamports to arbitrary recipients";

            loader_v4, allow_write, "Loader-v4 Write instructions",
                "Users can make the fee payer write arbitrary bytes into program accounts it is the authority for";

            loader_v4, allow_copy, "Loader-v4 Copy instructions",
                "Users can make the fee payer copy data between program accounts it is the authority for";

            loader_v4, allow_set_program_length, "Loader-v4 SetProgramLength instructions",
                "Users can make the fee payer resize program accounts. A drainage guard keeps refunds flowing back to the fee payer, but enabling this still lets users force-grow accounts and consume fee-payer rent";

            loader_v4, allow_deploy, "Loader-v4 Deploy instructions",
                "Users can make the fee payer deploy programs it is the authority for, committing rent";

            loader_v4, allow_retract, "Loader-v4 Retract instructions",
                "Users can make the fee payer retract deployed programs, taking them out of execution";

            loader_v4, allow_transfer_authority, "Loader-v4 TransferAuthority instructions",
                "Users can make the fee payer hand program authority to a different account. This can lead to complete loss of control and subsequent drainage";

            loader_v4, allow_finalize, "Loader-v4 Finalize instructions",
                "Users can make the fee payer finalize programs, making them immutable and forwarding to a next-version program";

            bpf_loader_upgradeable, allow_initialize_buffer, "BPF Loader Upgradeable InitializeBuffer instructions",
                "Users can make the fee payer the buffer authority on new buffers";

            bpf_loader_upgradeable, allow_write, "BPF Loader Upgradeable Write instructions",
                "Users can make the fee payer write arbitrary bytes into buffers it is the authority for";

            bpf_loader_upgradeable, allow_deploy_with_max_data_len, "BPF Loader Upgradeable DeployWithMaxDataLen instructions",
                "Users can make the fee payer fund + take upgrade authority on newly deployed programs, committing rent";

            bpf_loader_upgradeable, allow_upgrade, "BPF Loader Upgradeable Upgrade instructions",
                "Users can make the fee payer authorize program upgrades";

            bpf_loader_upgradeable, allow_set_authority, "BPF Loader Upgradeable SetAuthority instructions",
                "Users can make the fee payer hand buffer/program authority to a different account. This can lead to complete loss of control and subsequent drainage";

            bpf_loader_upgradeable, allow_set_authority_checked, "BPF Loader Upgradeable SetAuthorityChecked instructions",
                "Users can make the fee payer hand buffer/program authority to a different account (checked variant). Same drainage risk as SetAuthority";

            bpf_loader_upgradeable, allow_close, "BPF Loader Upgradeable Close instructions",
                "Users can make the fee payer close buffers/programs. The drainage guard rejects foreign-recipient closes, but enabling this still lets users force closure of buffers/programs Kora is the authority on";

            bpf_loader_upgradeable, allow_extend_program, "BPF Loader Upgradeable ExtendProgram instructions",
                "Users can make the fee payer fund extending program data accounts, draining rent";

            bpf_loader_upgradeable, allow_extend_program_checked, "BPF Loader Upgradeable ExtendProgramChecked instructions",
                "Users can make the fee payer authorize and/or fund extending program data accounts (checked variant requires authority signature)";

            bpf_loader_upgradeable, allow_migrate, "BPF Loader Upgradeable Migrate instructions",
                "Users can make the fee payer migrate programs from loader-v3 to loader-v4. Authority moves to a less-validated path on this Kora";
        }

        let only_via = &policy.system.create_account_only_via;
        match (policy.system.allow_create_account, only_via.is_empty()) {
            (true, true) => warnings.push(
                "⚠️  SECURITY: Fee payer policy allows System CreateAccount instructions \
                 (allow_create_account). Risk: Users can make the fee payer pay for arbitrary \
                 account creations. This can drain your fee payer account. Consider setting \
                 [validation.fee_payer_policy.system] allow_create_account=false, or restricting \
                 it with create_account_only_via."
                    .to_string(),
            ),
            (true, false) => warnings.push(format!(
                "⚠️  SECURITY: Fee payer policy allows account creation funded by the fee payer \
                 only inside a CPI from {only_via:?} (create_account_only_via). Risk: those \
                 programs decide which accounts the fee payer funds, bounded per request by \
                 max_allowed_lamports."
            )),
            (false, false) => warnings.push(
                "[validation.fee_payer_policy.system] create_account_only_via has no effect \
                 while allow_create_account=false: the fee payer cannot fund any account creation"
                    .to_string(),
            ),
            (false, true) => {}
        }

        // Check nonce policy separately (nested structure)
        macro_rules! check_nonce_policy {
        ($($field:ident, $description:expr, $risk:expr);* $(;)?) => {
            $(
                if policy.system.nonce.$field {
                    warnings.push(format!(
                        "⚠️  SECURITY: Fee payer policy allows {} (nonce.{}). \
                        Risk: {}. \
                        Consider setting [validation.fee_payer_policy.system.nonce] {}=false to prevent abuse.",
                        $description,
                        stringify!($field),
                        $risk,
                        stringify!($field)
                    ));
                }
            )*
        };
    }

        check_nonce_policy! {
            allow_initialize, "nonce account initialization",
                "Users can make the fee payer the authority of nonce accounts. This can create unexpected control relationships";

            allow_advance, "nonce account advancement",
                "Users can make the fee payer advance nonce accounts. This can be used to manipulate nonce states";

            allow_withdraw, "nonce account withdrawals",
                "Users can make the fee payer withdraw from nonce accounts. This can drain nonce account balances";

            allow_authorize, "nonce authority changes",
                "Users can make the fee payer transfer nonce authority. This can lead to loss of control over nonce accounts";
        }
    }
}
