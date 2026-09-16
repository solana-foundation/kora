#![cfg(test)]

use anyhow::Result;
use solana_address_lookup_table_interface::{
    instruction::derive_lookup_table_address,
    state::{AddressLookupTable, LookupTableMeta},
};
use solana_program_pack::Pack;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, program_option::COption, pubkey::Pubkey, rent::Rent,
    signature::Signer,
};
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use spl_token_2022_interface::{
    extension::{
        immutable_owner::ImmutableOwner,
        transfer_fee::{TransferFee, TransferFeeAmount, TransferFeeConfig},
        BaseStateWithExtensionsMut, ExtensionType, StateWithExtensionsMut,
    },
    state::{
        Account as Token2022Account, AccountState as Token2022AccountState, Mint as Token2022Mint,
    },
};
use std::borrow::Cow;
use surfpool_sdk::{cheatcodes::Cheatcodes, Surfnet};

use crate::common::{
    setup::{ts_auth_wallet, ts_free_wallet, TestAccountInfo},
    FeePayerPolicyMintTestHelper, FeePayerTestHelper, LookupTableHelper, RecipientTestHelper,
    SenderTestHelper, USDCMint2022TestHelper, USDCMintTestHelper,
};

const SOL_FUNDING: u64 = 10 * LAMPORTS_PER_SOL;
const TRANSFER_FEE_BASIS_POINTS: u16 = 100;
const MAXIMUM_TRANSFER_FEE: u64 = 1_000_000;

/// Only PDA seeds: no `CreateLookupTable` runs, so these need not be recent.
const LOOKUP_TABLE_SLOTS: [u64; 3] = [1, 2, 3];

struct Holder {
    owner: Pubkey,
    amount: u64,
}

/// Writes the same account state `TestAccountSetup::setup_all_accounts` creates.
pub fn seed_accounts(surfnet: &Surfnet) -> Result<TestAccountInfo> {
    let cheats = surfnet.cheatcodes();

    let sender_pubkey = SenderTestHelper::get_test_sender_keypair().pubkey();
    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let decimals = USDCMintTestHelper::get_test_usdc_mint_decimals();
    let mint_amount = 1_000_000 * 10_u64.pow(decimals as u32);

    for owner in
        [sender_pubkey, recipient_pubkey, fee_payer_pubkey, ts_auth_wallet(), ts_free_wallet()]
    {
        cheats.fund_sol(&owner, SOL_FUNDING)?;
    }

    let usdc_mint_pubkey = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let usdc_holders = [
        Holder { owner: sender_pubkey, amount: mint_amount },
        Holder { owner: recipient_pubkey, amount: 0 },
        Holder { owner: fee_payer_pubkey, amount: 0 },
        Holder { owner: ts_auth_wallet(), amount: mint_amount },
        Holder { owner: ts_free_wallet(), amount: mint_amount },
    ];
    let usdc_accounts =
        write_spl_mint(&cheats, &usdc_mint_pubkey, &sender_pubkey, decimals, &usdc_holders)?;

    let usdc_mint_2022_pubkey = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();
    let usdc_2022_holders = [
        Holder { owner: sender_pubkey, amount: mint_amount },
        Holder { owner: recipient_pubkey, amount: 0 },
        Holder { owner: fee_payer_pubkey, amount: 0 },
    ];
    let usdc_2022_accounts = write_token_2022_mint(
        &cheats,
        &usdc_mint_2022_pubkey,
        &sender_pubkey,
        decimals,
        true,
        &usdc_2022_holders,
    )?;

    let fee_payer_policy_mint_pubkey =
        FeePayerPolicyMintTestHelper::get_fee_payer_policy_mint_pubkey();
    let policy_holders = [
        Holder { owner: sender_pubkey, amount: mint_amount },
        Holder { owner: recipient_pubkey, amount: 0 },
        Holder { owner: fee_payer_pubkey, amount: 0 },
    ];
    let policy_accounts = write_spl_mint(
        &cheats,
        &fee_payer_policy_mint_pubkey,
        &fee_payer_pubkey,
        decimals,
        &policy_holders,
    )?;

    let fee_payer_policy_mint_2022_pubkey =
        FeePayerPolicyMintTestHelper::get_fee_payer_policy_mint_2022_pubkey();
    let policy_2022_accounts = write_token_2022_mint(
        &cheats,
        &fee_payer_policy_mint_2022_pubkey,
        &fee_payer_pubkey,
        decimals,
        false,
        &policy_holders,
    )?;

    let (allowed_lookup_table, disallowed_lookup_table, transaction_lookup_table) =
        write_lookup_tables(&cheats, &sender_pubkey)?;

    Ok(TestAccountInfo {
        fee_payer_pubkey,
        sender_pubkey,
        recipient_pubkey,
        usdc_mint_pubkey,
        sender_token_account: usdc_accounts[0],
        recipient_token_account: usdc_accounts[1],
        fee_payer_token_account: usdc_accounts[2],
        usdc_mint_2022_pubkey,
        sender_token_2022_account: usdc_2022_accounts[0],
        recipient_token_2022_account: usdc_2022_accounts[1],
        fee_payer_token_2022_account: usdc_2022_accounts[2],
        fee_payer_policy_mint_pubkey,
        fee_payer_policy_sender_token_account: policy_accounts[0],
        fee_payer_policy_recipient_token_account: policy_accounts[1],
        fee_payer_policy_fee_payer_token_account: policy_accounts[2],
        fee_payer_policy_mint_2022_pubkey,
        fee_payer_policy_sender_token_2022_account: policy_2022_accounts[0],
        fee_payer_policy_recipient_token_2022_account: policy_2022_accounts[1],
        fee_payer_policy_fee_payer_token_2022_account: policy_2022_accounts[2],
        allowed_lookup_table,
        disallowed_lookup_table,
        transaction_lookup_table,
    })
}

fn write_spl_mint(
    cheats: &Cheatcodes,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
    holders: &[Holder],
) -> Result<Vec<Pubkey>> {
    let state = spl_token_interface::state::Mint {
        mint_authority: COption::Some(*authority),
        supply: holders.iter().map(|holder| holder.amount).sum(),
        decimals,
        is_initialized: true,
        freeze_authority: COption::Some(*authority),
    };
    let mut data = vec![0u8; spl_token_interface::state::Mint::LEN];
    state.pack_into_slice(&mut data);
    write_account(cheats, mint, &data, &spl_token_interface::id())?;

    holders
        .iter()
        .map(|holder| {
            let account = spl_token_interface::state::Account {
                mint: *mint,
                owner: holder.owner,
                amount: holder.amount,
                state: spl_token_interface::state::AccountState::Initialized,
                ..Default::default()
            };
            let mut data = vec![0u8; spl_token_interface::state::Account::LEN];
            account.pack_into_slice(&mut data);

            let address = get_associated_token_address_with_program_id(
                &holder.owner,
                mint,
                &spl_token_interface::id(),
            );
            write_account(cheats, &address, &data, &spl_token_interface::id())?;
            Ok(address)
        })
        .collect()
}

fn write_token_2022_mint(
    cheats: &Cheatcodes,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
    transfer_fee: bool,
    holders: &[Holder],
) -> Result<Vec<Pubkey>> {
    let extensions: &[ExtensionType] =
        if transfer_fee { &[ExtensionType::TransferFeeConfig] } else { &[] };
    let mut data =
        vec![0u8; ExtensionType::try_calculate_account_len::<Token2022Mint>(extensions)?];
    {
        let mut state = StateWithExtensionsMut::<Token2022Mint>::unpack_uninitialized(&mut data)?;
        if transfer_fee {
            let fee = TransferFee {
                epoch: 0.into(),
                maximum_fee: MAXIMUM_TRANSFER_FEE.into(),
                transfer_fee_basis_points: TRANSFER_FEE_BASIS_POINTS.into(),
            };
            let config = state.init_extension::<TransferFeeConfig>(true)?;
            config.transfer_fee_config_authority = Some(*authority).try_into()?;
            config.withdraw_withheld_authority = Some(*authority).try_into()?;
            config.withheld_amount = 0.into();
            config.older_transfer_fee = fee;
            config.newer_transfer_fee = fee;
        }
        state.base = Token2022Mint {
            mint_authority: COption::Some(*authority),
            supply: holders.iter().map(|holder| holder.amount).sum(),
            decimals,
            is_initialized: true,
            freeze_authority: COption::Some(*authority),
        };
        state.pack_base();
        state.init_account_type()?;
    }
    write_account(cheats, mint, &data, &spl_token_2022_interface::id())?;

    holders
        .iter()
        .map(|holder| {
            let address = get_associated_token_address_with_program_id(
                &holder.owner,
                mint,
                &spl_token_2022_interface::id(),
            );
            write_account(
                cheats,
                &address,
                &token_2022_account_data(mint, holder, transfer_fee)?,
                &spl_token_2022_interface::id(),
            )?;
            Ok(address)
        })
        .collect()
}

/// The associated token account program always adds `ImmutableOwner`, and adds
/// `TransferFeeAmount` for a mint that charges a transfer fee.
fn token_2022_account_data(mint: &Pubkey, holder: &Holder, transfer_fee: bool) -> Result<Vec<u8>> {
    let mut extensions = vec![ExtensionType::ImmutableOwner];
    if transfer_fee {
        extensions.push(ExtensionType::TransferFeeAmount);
    }

    let mut data =
        vec![0u8; ExtensionType::try_calculate_account_len::<Token2022Account>(&extensions)?];
    let mut state = StateWithExtensionsMut::<Token2022Account>::unpack_uninitialized(&mut data)?;
    state.init_extension::<ImmutableOwner>(true)?;
    if transfer_fee {
        state.init_extension::<TransferFeeAmount>(true)?.withheld_amount = 0.into();
    }
    state.base = Token2022Account {
        mint: *mint,
        owner: holder.owner,
        amount: holder.amount,
        state: Token2022AccountState::Initialized,
        ..Default::default()
    };
    state.pack_base();
    state.init_account_type()?;

    Ok(data)
}

/// The `LookupTableMeta` defaults (`deactivation_slot` `u64::MAX`,
/// `last_extended_slot` 0) make the tables usable with no activation wait.
fn write_lookup_tables(
    cheats: &Cheatcodes,
    authority: &Pubkey,
) -> Result<(Pubkey, Pubkey, Pubkey)> {
    let contents = [
        vec![solana_system_interface::program::ID],
        vec![LookupTableHelper::get_test_disallowed_address()?],
        vec![USDCMintTestHelper::get_test_usdc_mint_pubkey(), spl_token_interface::ID],
    ];

    let mut addresses = Vec::with_capacity(contents.len());
    for (slot, table_addresses) in LOOKUP_TABLE_SLOTS.iter().zip(contents) {
        let (address, _) = derive_lookup_table_address(authority, *slot);
        let data = AddressLookupTable {
            meta: LookupTableMeta::new(*authority),
            addresses: Cow::Owned(table_addresses),
        }
        .serialize_for_tests()?;
        write_account(
            cheats,
            &address,
            &data,
            &solana_address_lookup_table_interface::program::id(),
        )?;
        addresses.push(address);
    }

    Ok((addresses[0], addresses[1], addresses[2]))
}

fn write_account(cheats: &Cheatcodes, address: &Pubkey, data: &[u8], owner: &Pubkey) -> Result<()> {
    cheats.set_account(address, Rent::default().minimum_balance(data.len()), data, owner)?;
    Ok(())
}
