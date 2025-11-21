/// Macro to validate system instructions with consistent pattern
macro_rules! validate_system {
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
