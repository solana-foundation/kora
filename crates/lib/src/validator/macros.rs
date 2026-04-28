/// Macro to validate system instructions with consistent pattern
macro_rules! validate_system {
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => $account:expr, $policy:expr, $name:expr, $extra:block) => {
        for instruction in $instructions.get(&ParsedSystemInstructionType::$type).unwrap_or(&vec![])
        {
            if let $pattern = instruction {
                if *$account == $self.fee_payer_pubkey {
                    if !$policy {
                        return Err(KoraError::InvalidTransaction(format!(
                            "Fee payer cannot be used for '{}'",
                            $name
                        )));
                    }
                    $extra
                }
            }
        }
    };
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => $account:expr, $policy:expr, $name:expr) => {
        for instruction in $instructions.get(&ParsedSystemInstructionType::$type).unwrap_or(&vec![])
        {
            if let $pattern = instruction {
                if *$account == $self.fee_payer_pubkey && !$policy {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        $name
                    )));
                }
            }
        }
    };
}

/// Macro to validate SPL/Token2022 instructions with is_2022 branching
macro_rules! validate_spl {
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => { $account:expr, $is_2022:expr }, $spl_policy:expr, $token2022_policy:expr, $name_spl:expr, $name_2022:expr) => {
        for instruction in $instructions.get(&ParsedSPLInstructionType::$type).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                let (allowed, name) = if *$is_2022 {
                    ($token2022_policy, $name_2022)
                } else {
                    ($spl_policy, $name_spl)
                };
                if *$account == $self.fee_payer_pubkey && !allowed {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        name
                    )));
                }
            }
        }
    };
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => { $account:expr, $multisig_signers:expr, $is_2022:expr }, $spl_policy:expr, $token2022_policy:expr, $name_spl:expr, $name_2022:expr) => {
        for instruction in $instructions.get(&ParsedSPLInstructionType::$type).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                let (allowed, name) = if *$is_2022 {
                    ($token2022_policy, $name_2022)
                } else {
                    ($spl_policy, $name_spl)
                };
                if (*$account == $self.fee_payer_pubkey
                    || $multisig_signers.contains(&$self.fee_payer_pubkey))
                    && !allowed
                {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        name
                    )));
                }
            }
        }
    };
}

/// Macro to validate SPL/Token2022 multisig instructions that check against a list of signers
macro_rules! validate_spl_multisig {
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => { $signers:expr, $is_2022:expr }, $spl_policy:expr, $token2022_policy:expr, $name_spl:expr, $name_2022:expr) => {
        for instruction in $instructions.get(&ParsedSPLInstructionType::$type).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                let (allowed, name) = if *$is_2022 {
                    ($token2022_policy, $name_2022)
                } else {
                    ($spl_policy, $name_spl)
                };
                // Check if fee payer is one of the signers
                if $signers.contains(&$self.fee_payer_pubkey) && !allowed {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        name
                    )));
                }
            }
        }
    };
}

/// Macro to validate ALT instructions with custom fee-payer matching logic.
/// The caller's `$fee_payer_used` expression is pre-evaluated and already references
/// `self.fee_payer_pubkey`, so the macro does not need to capture `self`.
macro_rules! validate_alt {
    ($instructions:expr, $type:ident, $pattern:pat => $fee_payer_used:expr, $policy:expr, $name:expr) => {
        for instruction in $instructions.get(&ParsedALTInstructionType::$type).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                if $fee_payer_used && !$policy {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        $name
                    )));
                }
            }
        }
    };
}

/// Macro to validate BPF Loader Upgradeable (loader-v3) instructions with custom
/// fee-payer matching logic. Same shape as `validate_alt!` — the caller's
/// `$fee_payer_used` expression is pre-evaluated against `self.fee_payer_pubkey`.
macro_rules! validate_bpf_loader_upgradeable {
    ($instructions:expr, $type:ident, $pattern:pat => $fee_payer_used:expr, $policy:expr, $name:expr) => {
        for instruction in
            $instructions.get(&ParsedBpfLoaderUpgradeableInstructionType::$type).unwrap_or(&vec![])
        {
            if let $pattern = instruction {
                if $fee_payer_used && !$policy {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        $name
                    )));
                }
            }
        }
    };
}

/// Macro to validate Loader-v4 instructions with custom fee-payer matching logic.
/// Same shape as `validate_alt!` — the caller's `$fee_payer_used` expression is
/// pre-evaluated against `self.fee_payer_pubkey` at the call site.
macro_rules! validate_loader_v4 {
    ($instructions:expr, $type:ident, $pattern:pat => $fee_payer_used:expr, $policy:expr, $name:expr) => {
        for instruction in
            $instructions.get(&ParsedLoaderV4InstructionType::$type).unwrap_or(&vec![])
        {
            if let $pattern = instruction {
                if $fee_payer_used && !$policy {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be used for '{}'",
                        $name
                    )));
                }
            }
        }
    };
}

/// Macro to validate Token2022-only instructions with custom fee-payer matching logic
macro_rules! validate_token2022 {
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => $fee_payer_used:expr, $message:expr) => {
        for instruction in $instructions.get(&ParsedSPLInstructionType::$type).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                if $fee_payer_used {
                    return Err(KoraError::InvalidTransaction($message.to_string()));
                }
            }
        }
    };
    ($self:expr, $instructions:expr, $type:ident, $pattern:pat => $fee_payer_used:expr, $policy:expr, $message:expr) => {
        for instruction in $instructions.get(&ParsedSPLInstructionType::$type).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                if $fee_payer_used && !$policy {
                    return Err(KoraError::InvalidTransaction($message.to_string()));
                }
            }
        }
    };
}
