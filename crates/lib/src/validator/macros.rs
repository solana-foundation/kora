/// Rejects every `$ty` instruction matching `$pattern` in which the fee payer is `$used`, unless
/// the policy flag after `unless` allows it. Without an `unless` clause the use is always rejected.
/// `token_policy(policy, is_2022).flag` picks the flag from `token_2022` or `spl_token`.
macro_rules! deny_fee_payer {
    (@reject $instructions:expr, $ty:path, $pattern:pat, $reject:expr, $message:expr) => {
        for instruction in $instructions.get(&$ty).unwrap_or(&vec![]) {
            if let $pattern = instruction {
                if $reject {
                    return Err(KoraError::InvalidTransaction($message.to_string()));
                }
            }
        }
    };
    ($instructions:expr, $ty:path, $pattern:pat => $used:expr,
        unless token_policy($policy:expr, $is_2022:expr).$flag:ident, $name:literal) => {
        deny_fee_payer!(@reject $instructions, $ty, $pattern,
            $used && !(if $is_2022 { $policy.token_2022.$flag } else { $policy.spl_token.$flag }),
            if $is_2022 {
                concat!("Fee payer cannot be used for 'Token2022 Token ", $name, "'")
            } else {
                concat!("Fee payer cannot be used for 'SPL Token ", $name, "'")
            })
    };
    ($instructions:expr, $ty:path, $pattern:pat => $used:expr, unless $allowed:expr, $message:expr) => {
        deny_fee_payer!(@reject $instructions, $ty, $pattern, $used && !$allowed, $message)
    };
    ($instructions:expr, $ty:path, $pattern:pat => $used:expr, $message:expr) => {
        deny_fee_payer!(@reject $instructions, $ty, $pattern, $used, $message)
    };
}
