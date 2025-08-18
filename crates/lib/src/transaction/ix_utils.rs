use solana_sdk::{instruction::CompiledInstruction, pubkey::Pubkey};

pub struct IxUtils;

impl IxUtils {
    pub fn get_account_key_if_present(
        ix: &CompiledInstruction,
        index: usize,
        account_keys: &[Pubkey],
    ) -> Option<Pubkey> {
        if ix.accounts.is_empty() {
            return None;
        }

        if index >= ix.accounts.len() {
            return None;
        }

        let idx = ix.accounts[index] as usize;

        if idx >= account_keys.len() {
            return None;
        }

        Some(account_keys[idx])
    }
}
